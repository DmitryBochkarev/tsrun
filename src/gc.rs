//! Mark-and-sweep garbage collection system.
//!
//! Objects are kept alive by being reachable from Guard roots through ownership edges.
//! Collection happens when guards are dropped, using a mark-and-sweep algorithm.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use rustc_hash::{FxHashMap, FxHashSet};

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

    /// Mark bit for mark-and-sweep collection
    marked: Cell<bool>,

    /// Whether this object is in the pool (dead)
    pooled: Cell<bool>,
}

impl<T> GcBox<T> {
    fn new(id: usize, data: T) -> Self {
        Self {
            id,
            data: RefCell::new(data),
            marked: Cell::new(false),
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

    /// Active guards: guard_id → set of directly guarded object indices
    guards: FxHashMap<usize, FxHashSet<usize>>,

    /// Ownership edges: owner_id → set of owned_ids
    /// Used during mark phase to traverse the object graph
    ownership_edges: FxHashMap<usize, FxHashSet<usize>>,

    /// Object ID to index mapping (for looking up objects by ID)
    id_to_index: FxHashMap<usize, usize>,

    /// Number of allocations since last collection
    allocs_since_gc: usize,

    /// Threshold for triggering collection (0 = never auto-collect)
    gc_threshold: usize,
}

/// Default threshold: collect after this many allocations
const DEFAULT_GC_THRESHOLD: usize = 100;

impl<T: Default + Reset> Space<T> {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            pool: Vec::new(),
            next_guard_id: 0,
            next_object_id: 0,
            guards: FxHashMap::default(),
            ownership_edges: FxHashMap::default(),
            id_to_index: FxHashMap::default(),
            allocs_since_gc: 0,
            gc_threshold: DEFAULT_GC_THRESHOLD,
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
            gc_box.marked.set(false);
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

        // Register object with guard (directly guarded)
        self.guards.entry(guard_id).or_default().insert(index);

        // Track allocations and maybe collect
        self.allocs_since_gc += 1;
        if self.gc_threshold > 0 && self.allocs_since_gc >= self.gc_threshold {
            self.collect();
        }

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
        self.guards.insert(id, FxHashSet::default());
        id
    }

    /// Remove a guard (collection happens lazily via threshold)
    fn remove_guard(&mut self, guard_id: usize) {
        self.guards.remove(&guard_id);
        // Don't collect immediately - let threshold handle it
        // This avoids O(n) work on every guard drop
    }

    /// Add a guard to an existing object (make it directly guarded by this guard)
    fn guard_object(&mut self, index: usize, guard_id: usize) {
        let Some(gc_box) = self.objects.get(index) else {
            return;
        };
        if gc_box.pooled.get() {
            return;
        }

        // Register object with guard
        self.guards.entry(guard_id).or_default().insert(index);
    }

    /// Remove a guard from a specific object
    fn unguard_object(&mut self, index: usize, guard_id: usize) {
        if let Some(guarded_objects) = self.guards.get_mut(&guard_id) {
            guarded_objects.remove(&index);
        }
        // Collection will happen when a guard is dropped
        // No immediate collection for performance
    }

    /// Mark phase: mark all objects reachable from roots
    fn mark(&mut self) {
        // Clear all marks first
        for gc_box in &self.objects {
            gc_box.marked.set(false);
        }

        // Collect all root object indices from all guards
        let roots: Vec<usize> = self
            .guards
            .values()
            .flat_map(|indices| indices.iter().copied())
            .collect();

        // Mark from each root
        for index in roots {
            self.mark_from(index);
        }
    }

    /// Recursively mark an object and all objects it owns
    fn mark_from(&mut self, index: usize) {
        let Some(gc_box) = self.objects.get(index) else {
            return;
        };

        // Already marked or pooled - stop
        if gc_box.marked.get() || gc_box.pooled.get() {
            return;
        }

        // Mark this object
        gc_box.marked.set(true);

        let object_id = gc_box.id;

        // Get owned objects and mark them recursively
        let owned_ids: Vec<usize> = self
            .ownership_edges
            .get(&object_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        for owned_id in owned_ids {
            if let Some(&owned_index) = self.id_to_index.get(&owned_id) {
                self.mark_from(owned_index);
            }
        }
    }

    /// Sweep phase: collect all unmarked objects
    fn sweep(&mut self) {
        let mut to_collect = Vec::new();

        for (index, gc_box) in self.objects.iter().enumerate() {
            if !gc_box.pooled.get() && !gc_box.marked.get() {
                to_collect.push(index);
            }
        }

        for index in to_collect {
            self.collect_object(index);
        }
    }

    /// Run mark-and-sweep collection
    fn collect(&mut self) {
        self.mark();
        self.sweep();
        self.allocs_since_gc = 0;
    }

    /// Force a collection (for testing or explicit cleanup)
    fn force_collect(&mut self) {
        self.collect();
    }

    /// Collect a single object (move to pool)
    fn collect_object(&mut self, index: usize) {
        let Some(gc_box) = self.objects.get(index) else {
            return;
        };

        if gc_box.pooled.get() {
            return;
        }

        let object_id = gc_box.id;

        // Remove from id_to_index
        self.id_to_index.remove(&object_id);

        // Remove all ownership edges involving this object
        self.ownership_edges.remove(&object_id);
        for owned_set in self.ownership_edges.values_mut() {
            owned_set.remove(&object_id);
        }

        // Mark as pooled and reset
        gc_box.pooled.set(true);
        gc_box.data.borrow_mut().reset();

        // Add to pool
        self.pool.push(index);
    }

    /// Establish ownership: owner owns owned
    fn own(&mut self, owner_id: usize, owned_id: usize) {
        // Don't allow self-ownership
        if owner_id == owned_id {
            return;
        }

        // Verify both objects exist and are alive
        let owner_alive = self.id_to_index.get(&owner_id).is_some_and(|&idx| {
            self.objects
                .get(idx)
                .is_some_and(|gc_box| !gc_box.pooled.get())
        });
        let owned_alive = self.id_to_index.get(&owned_id).is_some_and(|&idx| {
            self.objects
                .get(idx)
                .is_some_and(|gc_box| !gc_box.pooled.get())
        });

        if !owner_alive || !owned_alive {
            return;
        }

        // Add ownership edge
        self.ownership_edges
            .entry(owner_id)
            .or_default()
            .insert(owned_id);
    }

    /// Release ownership: owner no longer owns owned
    fn disown(&mut self, owner_id: usize, owned_id: usize) {
        if let Some(owned_set) = self.ownership_edges.get_mut(&owner_id) {
            owned_set.remove(&owned_id);
            if owned_set.is_empty() {
                self.ownership_edges.remove(&owner_id);
            }
        }
        // Collection will happen when a guard is dropped
        // No immediate collection for performance
    }

    /// Get statistics
    fn stats(&self) -> GcStats {
        let ownership_edge_count: usize = self.ownership_edges.values().map(|s| s.len()).sum();

        GcStats {
            total_objects: self.objects.len(),
            pooled_objects: self.pool.len(),
            live_objects: self.objects.len() - self.pool.len(),
            active_guards: self.guards.len(),
            ownership_edges: ownership_edge_count,
        }
    }

    /// Set the GC threshold (0 = disable automatic collection)
    fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold;
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

    /// Force a garbage collection cycle
    pub fn collect(&self) {
        self.inner.borrow_mut().force_collect();
    }

    /// Set the GC threshold (0 = disable automatic collection)
    pub fn set_gc_threshold(&self, threshold: usize) {
        self.inner.borrow_mut().set_gc_threshold(threshold);
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
/// Objects are kept alive as long as they are reachable from at least one guard
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
    /// Makes the object directly reachable from this guard.
    pub fn guard(&self, obj: &Gc<T>) {
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().guard_object(obj.index, self.id);
        }
    }

    /// Remove this guard from an object.
    /// Object may be collected if it's no longer reachable.
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
        // Notify space to remove this guard and run collection
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
    /// Other becomes reachable through self.
    pub fn own(&self, other: &Gc<T>, heap: &Heap<T>) {
        heap.inner.borrow_mut().own(self.id, other.id);
    }

    /// Release ownership: self no longer owns other.
    /// Other may be collected if no longer reachable.
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

        heap.collect(); // Need explicit collect since we use threshold-based GC
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

        heap.collect(); // Force collection
        // Object should be in pool now
        assert_eq!(heap.stats().pooled_objects, 1);

        // Allocate again - should reuse from pool
        let guard = heap.create_guard();
        heap.set_gc_threshold(0); // Disable threshold for this test
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

        heap.collect(); // Force collection
        assert_eq!(heap.stats().live_objects, 2);

        // Disown B - it should be collected after GC
        a.disown(&b, &heap);
        heap.collect(); // Force collection

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
        heap.collect();
        assert_eq!(heap.stats().live_objects, 3); // C alive via A and B

        a.disown(&c, &heap);
        heap.collect();
        assert_eq!(heap.stats().live_objects, 3); // C still alive via B

        b.disown(&c, &heap);
        heap.collect();
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
        heap.collect();

        // A should still be alive (through B which is still directly guarded)
        assert_eq!(heap.stats().live_objects, 2);

        // Unguard B from guard - both should be collected (cycle is unreachable)
        guard.unguard(&b);
        heap.collect();

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

        // Add guard2 to A - B should survive via A even if guard1 is dropped
        guard2.guard(&a);

        // Drop guard1 - both should survive via guard2 → A → B
        drop(guard1);
        heap.collect();

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
        heap.collect();

        // All should still be alive (C through B, B through A)
        assert_eq!(heap.stats().live_objects, 3);

        // Unguard A - all should be collected (no path to any root)
        guard.unguard(&a);
        heap.collect();

        assert_eq!(heap.stats().live_objects, 0);
    }

    #[test]
    fn test_diamond_ownership() {
        let heap: Heap<TestObj> = Heap::new();
        let guard = heap.create_guard();

        // Diamond: A owns B and C, both B and C own D
        let a = guard.alloc();
        let b = guard.alloc();
        let c = guard.alloc();
        let d = guard.alloc();

        a.own(&b, &heap);
        a.own(&c, &heap);
        b.own(&d, &heap);
        c.own(&d, &heap);

        // Unguard all except A
        guard.unguard(&b);
        guard.unguard(&c);
        guard.unguard(&d);
        heap.collect();

        // All should still be alive via A
        assert_eq!(heap.stats().live_objects, 4);

        // Remove one path to D (B → D)
        b.disown(&d, &heap);
        heap.collect();

        // D should still be alive via C
        assert_eq!(heap.stats().live_objects, 4);

        // Remove other path to D (C → D)
        c.disown(&d, &heap);
        heap.collect();

        // D should now be collected
        assert_eq!(heap.stats().live_objects, 3);
    }

    #[test]
    fn test_multiple_guards_same_object() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();

        let obj = guard1.alloc();
        guard2.guard(&obj);

        assert_eq!(heap.stats().live_objects, 1);

        drop(guard1);
        heap.collect();
        assert_eq!(heap.stats().live_objects, 1); // Still alive via guard2

        drop(guard2);
        heap.collect();
        assert_eq!(heap.stats().live_objects, 0); // Now collected
    }
}
