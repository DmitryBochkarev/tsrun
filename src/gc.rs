//! Guard-based garbage collection system.
//!
//! Objects are kept alive by being transitively owned from Guard roots.
//! Each object tracks *why* it's guarded via `GuardSource`.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashSet;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

// ============================================================================
// GuardSource - tracks why an object is guarded
// ============================================================================

/// Tracks the reason why an object is guarded.
///
/// Each object maintains a set of `GuardSource` entries. When the set becomes
/// empty, the object can be collected.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum GuardSource {
    /// Directly guarded by a root guard (from `guard.alloc()` or `guard.guard()`)
    Direct { guard: usize },
    /// Guarded through ownership by another object
    Through { object_id: usize, guard: usize },
}

impl GuardSource {
    /// Get the root guard ID regardless of how we got it
    pub fn guard_id(&self) -> usize {
        match self {
            GuardSource::Direct { guard } => *guard,
            GuardSource::Through { guard, .. } => *guard,
        }
    }
}

// ============================================================================
// Reset trait - for pooling objects
// ============================================================================

/// Trait for types that can be reset to a clean state for pooling.
///
/// When an object is collected, it's reset and placed in a pool for reuse.
pub trait Reset: Default {
    /// Reset object to clean state (equivalent to Default but in-place)
    fn reset(&mut self);
}

// ============================================================================
// GcBox - the internal storage for GC-managed objects
// ============================================================================

/// Internal storage for a GC-managed object.
struct GcBox<T> {
    /// Unique object ID (stable across lifetime, unlike index)
    id: usize,

    /// The actual data
    data: RefCell<T>,

    /// Set of reasons why this object is guarded.
    /// Empty set means object can be collected.
    guard_sources: RefCell<HashSet<GuardSource>>,

    /// Whether this object is in the pool (dead)
    pooled: Cell<bool>,
}

impl<T> GcBox<T> {
    fn new(id: usize, data: T) -> Self {
        Self {
            id,
            data: RefCell::new(data),
            guard_sources: RefCell::new(HashSet::new()),
            pooled: Cell::new(false),
        }
    }
}

// ============================================================================
// Space - the internal memory arena
// ============================================================================

/// Internal memory arena that manages all allocations.
/// Not exposed directly - accessed through `Heap<T>`.
struct Space<T> {
    /// All allocated objects (including pooled).
    /// Using Box to ensure stable addresses when Vec reallocates.
    objects: Vec<Box<GcBox<T>>>,

    /// Free list of pooled object indices
    pool: Vec<usize>,

    /// Next guard ID
    next_guard_id: usize,

    /// Next object ID
    next_object_id: usize,

    /// Active guards: guard_id → set of object indices
    /// Used for fast lookup when a guard is dropped
    guards: std::collections::HashMap<usize, HashSet<usize>>,

    /// Ownership edges: (owner_id, owned_id) pairs
    /// Used to propagate guard changes through ownership graph
    ownership_edges: HashSet<(usize, usize)>,

    /// Object ID to index mapping (for looking up objects by ID)
    id_to_index: std::collections::HashMap<usize, usize>,
}

impl<T: Default + Reset> Space<T> {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            pool: Vec::new(),
            next_guard_id: 0,
            next_object_id: 0,
            guards: std::collections::HashMap::new(),
            ownership_edges: HashSet::new(),
            id_to_index: std::collections::HashMap::new(),
        }
    }

    /// Allocate a new object with the given guard
    fn alloc_internal(&mut self, guard_id: usize) -> Gc<T> {
        let (index, id, ptr) = if let Some(index) = self.pool.pop() {
            // Reuse from pool - safe because pool contains valid indices
            let gc_box = self.objects.get_mut(index).unwrap_or_else(|| {
                // Pool should only contain valid indices, this is an internal error
                #[allow(clippy::panic)]
                {
                    panic!("GC internal error: invalid pool index {index}")
                }
            });
            gc_box.data.borrow_mut().reset();
            gc_box.guard_sources.borrow_mut().clear();
            gc_box.pooled.set(false);

            // Assign new ID for reused object
            let id = self.next_object_id;
            self.next_object_id += 1;
            gc_box.id = id;

            self.id_to_index.insert(id, index);

            // Safe: Box has stable address
            let ptr = NonNull::from(gc_box.as_ref());
            (index, id, ptr)
        } else {
            // Allocate new - use Box for stable address
            let id = self.next_object_id;
            self.next_object_id += 1;

            let gc_box = Box::new(GcBox::new(id, T::default()));
            let ptr = NonNull::from(gc_box.as_ref());
            let index = self.objects.len();
            self.objects.push(gc_box);

            self.id_to_index.insert(id, index);

            (index, id, ptr)
        };

        // Add Direct guard source - safe because we just validated index above
        if let Some(gc_box) = self.objects.get(index) {
            gc_box
                .guard_sources
                .borrow_mut()
                .insert(GuardSource::Direct { guard: guard_id });
        }

        // Register object with guard
        self.guards.entry(guard_id).or_default().insert(index);

        Gc {
            id,
            index,
            ptr,
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new guard ID
    fn create_guard_id(&mut self) -> usize {
        let id = self.next_guard_id;
        self.next_guard_id += 1;
        self.guards.insert(id, HashSet::new());
        id
    }

    /// Remove a guard from all objects and collect those with no guards
    fn remove_guard(&mut self, guard_id: usize) {
        // Get all objects that have this guard directly
        let Some(object_indices) = self.guards.remove(&guard_id) else {
            return;
        };

        // For each object, unguard it
        let indices: Vec<usize> = object_indices.into_iter().collect();
        for index in indices {
            self.unguard_object(index, guard_id);
        }
    }

    /// Unguard a specific object from a guard
    fn unguard_object(&mut self, index: usize, guard_id: usize) {
        let object_id = {
            let Some(gc_box) = self.objects.get(index) else {
                return;
            };
            if gc_box.pooled.get() {
                return;
            }

            // Remove Direct{guard} from object
            gc_box
                .guard_sources
                .borrow_mut()
                .remove(&GuardSource::Direct { guard: guard_id });

            gc_box.id
        };

        // Check if object still has this guard through a valid (non-circular) path
        let still_has_guard = self.has_valid_guard_path(object_id, guard_id, &mut HashSet::new());

        // Only propagate if the object completely lost this guard
        if !still_has_guard {
            self.propagate_guard_remove(object_id, guard_id);
        }

        // Re-check if object should be collected
        let should_collect = if let Some(&idx) = self.id_to_index.get(&object_id) {
            self.objects.get(idx).is_some_and(|gc_box| {
                !gc_box.pooled.get() && gc_box.guard_sources.borrow().is_empty()
            })
        } else {
            false
        };

        // If object has no sources left, collect it
        if should_collect {
            self.collect(index);
        }
    }

    /// Check if an object has a valid (non-circular) path to a Direct guard
    fn has_valid_guard_path(
        &self,
        object_id: usize,
        guard_id: usize,
        visited: &mut HashSet<usize>,
    ) -> bool {
        // Cycle detection
        if !visited.insert(object_id) {
            return false;
        }

        let index = match self.id_to_index.get(&object_id) {
            Some(&idx) => idx,
            None => return false,
        };

        let Some(gc_box) = self.objects.get(index) else {
            return false;
        };
        if gc_box.pooled.get() {
            return false;
        }

        for source in gc_box.guard_sources.borrow().iter() {
            match source {
                GuardSource::Direct { guard } if *guard == guard_id => {
                    return true; // Found a direct path to the guard
                }
                GuardSource::Through {
                    object_id: through_id,
                    guard,
                } if *guard == guard_id => {
                    // Check if the "through" object has a valid path
                    if self.has_valid_guard_path(*through_id, guard_id, visited) {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Add a guard to an object
    fn guard_object(&mut self, index: usize, guard_id: usize) {
        let Some(gc_box) = self.objects.get(index) else {
            return;
        };
        if gc_box.pooled.get() {
            return;
        }

        let object_id = gc_box.id;

        // Add Direct{guard} to object
        gc_box
            .guard_sources
            .borrow_mut()
            .insert(GuardSource::Direct { guard: guard_id });

        // Register object with guard
        self.guards.entry(guard_id).or_default().insert(index);

        // Propagate: add Through{obj, guard} to all objects this object owns
        self.propagate_guard_add(object_id, guard_id);
    }

    /// Propagate guard addition through ownership graph
    fn propagate_guard_add(&mut self, owner_id: usize, guard_id: usize) {
        let mut visited = HashSet::new();
        self.propagate_guard_add_inner(owner_id, guard_id, &mut visited);
    }

    fn propagate_guard_add_inner(
        &mut self,
        owner_id: usize,
        guard_id: usize,
        visited: &mut HashSet<usize>,
    ) {
        // Avoid infinite recursion in cycles
        if !visited.insert(owner_id) {
            return;
        }

        // Find all objects owned by this owner
        let owned_ids: Vec<usize> = self
            .ownership_edges
            .iter()
            .filter(|(oid, _)| *oid == owner_id)
            .map(|(_, owned_id)| *owned_id)
            .collect();

        for owned_id in owned_ids {
            if let Some(&index) = self.id_to_index.get(&owned_id) {
                if let Some(gc_box) = self.objects.get(index) {
                    if !gc_box.pooled.get() {
                        gc_box
                            .guard_sources
                            .borrow_mut()
                            .insert(GuardSource::Through {
                                object_id: owner_id,
                                guard: guard_id,
                            });
                        // Recursively propagate
                        self.propagate_guard_add_inner(owned_id, guard_id, visited);
                    }
                }
            }
        }
    }

    /// Propagate guard removal through ownership graph
    fn propagate_guard_remove(&mut self, owner_id: usize, guard_id: usize) {
        let mut visited = HashSet::new();
        self.propagate_guard_remove_inner(owner_id, guard_id, &mut visited);
    }

    fn propagate_guard_remove_inner(
        &mut self,
        owner_id: usize,
        guard_id: usize,
        visited: &mut HashSet<usize>,
    ) {
        // Avoid infinite recursion in cycles
        if !visited.insert(owner_id) {
            return;
        }

        // Find all objects owned by this owner
        let owned_ids: Vec<usize> = self
            .ownership_edges
            .iter()
            .filter(|(oid, _)| *oid == owner_id)
            .map(|(_, owned_id)| *owned_id)
            .collect();

        for owned_id in owned_ids {
            let (still_has_guard, should_collect) =
                if let Some(&index) = self.id_to_index.get(&owned_id) {
                    if let Some(gc_box) = self.objects.get(index) {
                        if !gc_box.pooled.get() {
                            gc_box
                                .guard_sources
                                .borrow_mut()
                                .remove(&GuardSource::Through {
                                    object_id: owner_id,
                                    guard: guard_id,
                                });
                            // Check if object still has this guard through another path
                            let still_has_guard = gc_box
                                .guard_sources
                                .borrow()
                                .iter()
                                .any(|s| s.guard_id() == guard_id);
                            (still_has_guard, gc_box.guard_sources.borrow().is_empty())
                        } else {
                            (true, false)
                        }
                    } else {
                        (true, false)
                    }
                } else {
                    (true, false)
                };

            // Only propagate if the object completely lost this guard
            if !still_has_guard {
                self.propagate_guard_remove_inner(owned_id, guard_id, visited);
            }

            // If object has no sources left, collect it
            if should_collect {
                if let Some(&index) = self.id_to_index.get(&owned_id) {
                    self.collect(index);
                }
            }
        }
    }

    /// Collect an object (move to pool)
    fn collect(&mut self, index: usize) {
        // First check if already pooled
        let object_id = {
            let Some(gc_box) = self.objects.get(index) else {
                return;
            };
            if gc_box.pooled.get() {
                return;
            }
            gc_box.id
        };

        // Remove from id_to_index
        self.id_to_index.remove(&object_id);

        // Remove all ownership edges involving this object
        self.ownership_edges
            .retain(|(owner, owned)| *owner != object_id && *owned != object_id);

        // Mark as pooled and reset
        if let Some(gc_box) = self.objects.get(index) {
            gc_box.pooled.set(true);
            gc_box.data.borrow_mut().reset();
            gc_box.guard_sources.borrow_mut().clear();
        }

        // Add to pool
        self.pool.push(index);
    }

    /// Establish ownership: owner owns owned
    fn own(&mut self, owner_id: usize, owned_id: usize) {
        // Don't allow self-ownership
        if owner_id == owned_id {
            return;
        }

        // Add edge
        if !self.ownership_edges.insert((owner_id, owned_id)) {
            // Edge already exists
            return;
        }

        // Get owner's guards and propagate to owned
        let owner_index = match self.id_to_index.get(&owner_id) {
            Some(&idx) => idx,
            None => return,
        };

        let guard_ids: Vec<usize> = self
            .objects
            .get(owner_index)
            .map(|gc_box| {
                gc_box
                    .guard_sources
                    .borrow()
                    .iter()
                    .map(|s| s.guard_id())
                    .collect()
            })
            .unwrap_or_default();

        // For each guard the owner has, add Through{owner, guard} to owned
        if let Some(&owned_index) = self.id_to_index.get(&owned_id) {
            if let Some(owned_box) = self.objects.get(owned_index) {
                for guard_id in &guard_ids {
                    owned_box
                        .guard_sources
                        .borrow_mut()
                        .insert(GuardSource::Through {
                            object_id: owner_id,
                            guard: *guard_id,
                        });
                }
            }
        }

        // Propagate to owned's children
        for guard_id in guard_ids {
            self.propagate_guard_add(owned_id, guard_id);
        }
    }

    /// Release ownership: owner no longer owns owned
    fn disown(&mut self, owner_id: usize, owned_id: usize) {
        // Remove edge
        if !self.ownership_edges.remove(&(owner_id, owned_id)) {
            // Edge didn't exist
            return;
        }

        // Get owner's guards
        let owner_index = match self.id_to_index.get(&owner_id) {
            Some(&idx) => idx,
            None => return,
        };

        let guard_ids: Vec<usize> = self
            .objects
            .get(owner_index)
            .map(|gc_box| {
                gc_box
                    .guard_sources
                    .borrow()
                    .iter()
                    .map(|s| s.guard_id())
                    .collect()
            })
            .unwrap_or_default();

        // For each guard the owner has, remove Through{owner, guard} from owned
        let should_collect = if let Some(&owned_index) = self.id_to_index.get(&owned_id) {
            if let Some(gc_box) = self.objects.get(owned_index) {
                for guard_id in &guard_ids {
                    gc_box
                        .guard_sources
                        .borrow_mut()
                        .remove(&GuardSource::Through {
                            object_id: owner_id,
                            guard: *guard_id,
                        });
                }
                gc_box.guard_sources.borrow().is_empty()
            } else {
                false
            }
        } else {
            false
        };

        // Propagate removal to owned's children
        for guard_id in &guard_ids {
            self.propagate_guard_remove(owned_id, *guard_id);
        }

        // If owned has no sources left, collect it
        if should_collect {
            if let Some(&owned_index) = self.id_to_index.get(&owned_id) {
                self.collect(owned_index);
            }
        }
    }

    /// Get statistics
    fn stats(&self) -> GcStats {
        GcStats {
            total_objects: self.objects.len(),
            pooled_objects: self.pool.len(),
            live_objects: self.objects.len() - self.pool.len(),
            active_guards: self.guards.len(),
            ownership_edges: self.ownership_edges.len(),
        }
    }
}

// ============================================================================
// Heap - the public wrapper
// ============================================================================

/// A wrapper around the GC space that provides the public API.
/// This is the main entry point for using the GC.
pub struct Heap<T> {
    inner: Rc<RefCell<Space<T>>>,
}

impl<T: Default + Reset> Heap<T> {
    /// Create a new heap
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Space::new())),
        }
    }

    /// Create a new guard (root)
    pub fn create_guard(&self) -> Guard<T> {
        let id = self.inner.borrow_mut().create_guard_id();
        Guard {
            id,
            space: Rc::downgrade(&self.inner),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> GcStats {
        self.inner.borrow().stats()
    }
}

impl<T: Default + Reset> Default for Heap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Heap<T> {
    fn clone(&self) -> Self {
        Heap {
            inner: self.inner.clone(),
        }
    }
}

// ============================================================================
// Guard - root anchor for objects
// ============================================================================

/// A root anchor that keeps objects alive.
///
/// Objects are kept alive as long as they belong to at least one guard
/// (directly or through ownership).
pub struct Guard<T: Default + Reset> {
    /// Unique identifier
    id: usize,

    /// Weak reference back to space for allocation and cleanup
    space: Weak<RefCell<Space<T>>>,
}

impl<T: Default + Reset> Guard<T> {
    /// Allocate a new object guarded by this guard.
    /// Returns a default T (either fresh or reset from pool).
    ///
    /// # Panics
    /// Panics if the Heap has been dropped while the guard is still alive.
    /// This should never happen in normal usage.
    pub fn alloc(&self) -> Gc<T> {
        let space = self.space.upgrade().unwrap_or_else(|| {
            #[allow(clippy::panic)]
            {
                panic!("GC error: Heap dropped while guard is still alive")
            }
        });
        let result = space.borrow_mut().alloc_internal(self.id);
        result
    }

    /// Add this guard to an existing object.
    /// Propagates to all objects owned by this object.
    pub fn guard(&self, obj: &Gc<T>) {
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().guard_object(obj.index, self.id);
        }
    }

    /// Remove this guard from an object.
    /// Propagates removal to all objects owned by this object.
    /// Object is collected if it has no remaining guard sources.
    pub fn unguard(&self, obj: &Gc<T>) {
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().unguard_object(obj.index, self.id);
        }
    }

    /// Get the guard's unique ID
    pub fn id(&self) -> usize {
        self.id
    }
}

impl<T: Default + Reset> Drop for Guard<T> {
    fn drop(&mut self) {
        // Notify space to remove this guard from all objects
        // If space is already dropped, nothing to do
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().remove_guard(self.id);
        }
    }
}

// ============================================================================
// Gc - smart pointer to GC-managed object
// ============================================================================

/// A smart pointer to a GC-managed object.
pub struct Gc<T> {
    /// Unique object ID (for ownership tracking)
    id: usize,

    /// Index into Space.objects (for fast access)
    index: usize,

    /// Pointer to the GcBox for fast access
    ptr: NonNull<GcBox<T>>,

    /// Marker for the type
    _marker: std::marker::PhantomData<T>,
}

impl<T> Gc<T> {
    /// Borrow the inner data immutably
    pub fn borrow(&self) -> Ref<'_, T> {
        unsafe { self.ptr.as_ref().data.borrow() }
    }

    /// Borrow the inner data mutably
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        unsafe { self.ptr.as_ref().data.borrow_mut() }
    }

    /// Get the object's unique ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Check if two Gc pointers point to the same object
    pub fn ptr_eq(a: &Gc<T>, b: &Gc<T>) -> bool {
        a.id == b.id
    }
}

impl<T: Default + Reset> Gc<T> {
    /// Establish ownership: self owns other.
    /// Other inherits all of self's guards.
    pub fn own(&self, other: &Gc<T>, heap: &Heap<T>) {
        heap.inner.borrow_mut().own(self.id, other.id);
    }

    /// Release ownership: self no longer owns other.
    /// Other loses guards that came through self.
    pub fn disown(&self, other: &Gc<T>, heap: &Heap<T>) {
        heap.inner.borrow_mut().disown(self.id, other.id);
    }
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Gc<T> {}

impl<T> std::fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gc")
            .field("id", &self.id)
            .field("index", &self.index)
            .finish()
    }
}

// ============================================================================
// GcStats - statistics about the GC
// ============================================================================

/// Statistics about the garbage collector
#[derive(Debug, Clone)]
pub struct GcStats {
    /// Total number of GcBox slots (including pooled)
    pub total_objects: usize,
    /// Number of objects in the pool (available for reuse)
    pub pooled_objects: usize,
    /// Number of live objects
    pub live_objects: usize,
    /// Number of active guards
    pub active_guards: usize,
    /// Number of ownership edges
    pub ownership_edges: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test type
    #[derive(Default, Debug, PartialEq)]
    struct TestObj {
        value: i32,
    }

    impl Reset for TestObj {
        fn reset(&mut self) {
            self.value = 0;
        }
    }

    #[test]
    fn test_basic_alloc() {
        let heap: Heap<TestObj> = Heap::new();
        let guard = heap.create_guard();
        let obj = guard.alloc();

        assert_eq!(obj.borrow().value, 0);
        obj.borrow_mut().value = 42;
        assert_eq!(obj.borrow().value, 42);

        let stats = heap.stats();
        assert_eq!(stats.live_objects, 1);
        assert_eq!(stats.pooled_objects, 0);
    }

    #[test]
    fn test_guard_drop_collects() {
        let heap: Heap<TestObj> = Heap::new();

        {
            let guard = heap.create_guard();
            let _obj = guard.alloc();

            assert_eq!(heap.stats().live_objects, 1);
        } // guard dropped here

        assert_eq!(heap.stats().live_objects, 0);
        assert_eq!(heap.stats().pooled_objects, 1);
    }

    #[test]
    fn test_pool_reuse() {
        let heap: Heap<TestObj> = Heap::new();

        {
            let guard = heap.create_guard();
            let obj = guard.alloc();
            obj.borrow_mut().value = 42;
        }

        // Object should be in pool now
        assert_eq!(heap.stats().pooled_objects, 1);

        // Allocate again - should reuse from pool
        let guard = heap.create_guard();
        let obj = guard.alloc();

        // Value should be reset
        assert_eq!(obj.borrow().value, 0);
        assert_eq!(heap.stats().pooled_objects, 0);
        assert_eq!(heap.stats().total_objects, 1);
    }

    #[test]
    fn test_ownership_keeps_alive() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();

        let a = guard1.alloc();
        let b = guard2.alloc();

        a.borrow_mut().value = 1;
        b.borrow_mut().value = 2;

        // A owns B
        a.own(&b, &heap);

        // Drop guard2 - B should survive because A owns it
        drop(guard2);

        assert_eq!(heap.stats().live_objects, 2);
        assert_eq!(b.borrow().value, 2);
    }

    #[test]
    fn test_disown_allows_collection() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();

        let a = guard1.alloc();
        let b = guard2.alloc();

        a.own(&b, &heap);
        drop(guard2);

        assert_eq!(heap.stats().live_objects, 2);

        // Disown B - it should be collected
        a.disown(&b, &heap);

        assert_eq!(heap.stats().live_objects, 1);
        assert_eq!(heap.stats().pooled_objects, 1);
    }

    #[test]
    fn test_multiple_owners() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();
        let guard3 = heap.create_guard();

        let a = guard1.alloc();
        let b = guard2.alloc();
        let c = guard3.alloc();

        // Both A and B own C
        a.own(&c, &heap);
        b.own(&c, &heap);

        drop(guard3);
        assert_eq!(heap.stats().live_objects, 3); // C alive via A and B

        a.disown(&c, &heap);
        assert_eq!(heap.stats().live_objects, 3); // C still alive via B

        b.disown(&c, &heap);
        assert_eq!(heap.stats().live_objects, 2); // C collected
    }

    #[test]
    fn test_cycle_collection() {
        let heap: Heap<TestObj> = Heap::new();
        let guard = heap.create_guard();

        let a = guard.alloc();
        let b = guard.alloc();

        a.borrow_mut().value = 1;
        b.borrow_mut().value = 2;

        // Create cycle: A owns B, B owns A
        a.own(&b, &heap);
        b.own(&a, &heap);

        assert_eq!(heap.stats().live_objects, 2);
        assert_eq!(heap.stats().ownership_edges, 2);

        // Unguard A from guard
        guard.unguard(&a);

        // A should still be alive (through B which has Direct{guard})
        assert_eq!(heap.stats().live_objects, 2);

        // Unguard B from guard - both should be collected
        guard.unguard(&b);

        assert_eq!(heap.stats().live_objects, 0);
        assert_eq!(heap.stats().pooled_objects, 2);
    }

    #[test]
    fn test_guard_add_propagates() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();

        let a = guard1.alloc();
        let b = guard1.alloc();

        a.own(&b, &heap);

        // Add guard2 to A - should propagate to B
        guard2.guard(&a);

        // Drop guard1 - both should survive via guard2
        drop(guard1);

        assert_eq!(heap.stats().live_objects, 2);
    }

    #[test]
    fn test_transitive_ownership() {
        let heap: Heap<TestObj> = Heap::new();
        let guard = heap.create_guard();

        let a = guard.alloc();
        let b = guard.alloc();
        let c = guard.alloc();

        // A → B → C
        a.own(&b, &heap);
        b.own(&c, &heap);

        // Unguard B and C from direct guard
        guard.unguard(&b);
        guard.unguard(&c);

        // All should still be alive (C through B, B through A)
        assert_eq!(heap.stats().live_objects, 3);

        // Unguard A - all should be collected (no path to any Direct guard)
        guard.unguard(&a);

        assert_eq!(heap.stats().live_objects, 0);
    }
}
