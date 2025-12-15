//! Mark-and-sweep garbage collection system.
//!
//! Objects are kept alive by being reachable from Guard roots through ownership edges.
//! Collection happens when guards are dropped, using a mark-and-sweep algorithm.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

use rustc_hash::FxHashMap;

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

    /// Guard count (number of guards directly protecting this object)
    guard_count: Cell<usize>,

    /// Reference count (ownership edges pointing to this object)
    ref_count: Cell<usize>,

    /// Mark bit for mark-and-sweep collection (for cycle detection)
    marked: Cell<bool>,

    /// Whether this object is in the pool (dead)
    pooled: Cell<bool>,
}

impl<T> GcBox<T> {
    fn new(id: usize, data: T) -> Self {
        Self {
            id,
            data: RefCell::new(data),
            guard_count: Cell::new(0),
            ref_count: Cell::new(0),
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
    /// Chunks of allocated objects. Each chunk has fixed capacity.
    /// Inner vecs never reallocate, ensuring stable pointers.
    chunks: Vec<Vec<GcBox<T>>>,

    /// Capacity of each chunk (fixed at initialization)
    chunk_capacity: usize,

    /// Free list of pooled object pointers
    pool: Vec<NonNull<GcBox<T>>>,

    /// Next object ID
    next_object_id: usize,

    /// Ownership edges: owner_id → Vec of (owned_id, owned_ptr)
    /// Stores pointers directly to avoid HashMap lookups during mark phase.
    ownership_edges: FxHashMap<usize, Vec<(usize, NonNull<GcBox<T>>)>>,

    /// Object ID to pointer mapping (for looking up objects by ID)
    /// Also serves as the set of all live (non-pooled) objects.
    id_to_ptr: FxHashMap<usize, NonNull<GcBox<T>>>,

    /// Net allocations: incremented on alloc, decremented on dealloc
    /// GC triggers when this exceeds threshold
    net_allocs: isize,

    /// Threshold for triggering collection (0 = never auto-collect)
    gc_threshold: isize,
}

/// Default threshold: collect after this many net allocations
/// Higher threshold = less frequent GC = better throughput but more memory
const DEFAULT_GC_THRESHOLD: usize = 100;

/// Default chunk capacity: objects per chunk
const DEFAULT_CHUNK_CAPACITY: usize = 256;

impl<T: Default + Reset> Space<T> {
    fn new() -> Self {
        Self::with_chunk_capacity(DEFAULT_CHUNK_CAPACITY)
    }

    fn with_chunk_capacity(chunk_capacity: usize) -> Self {
        Self {
            chunks: Vec::new(),
            chunk_capacity,
            pool: Vec::new(),
            next_object_id: 0,
            ownership_edges: FxHashMap::default(),
            id_to_ptr: FxHashMap::default(),
            net_allocs: 0,
            gc_threshold: DEFAULT_GC_THRESHOLD as isize,
        }
    }

    /// Allocate a new object (starts with ref_count = 1 for the allocating guard)
    fn alloc_internal(&mut self) -> Gc<T> {
        let (id, ptr) = if let Some(ptr) = self.pool.pop() {
            // Reuse from pool - safe because pool contains valid pointers
            // Safety: ptr came from our chunks which have stable addresses
            let gc_box = unsafe { ptr.as_ptr().as_mut() };
            let gc_box = match gc_box {
                Some(b) => b,
                None => {
                    #[allow(clippy::panic)]
                    {
                        panic!("GC internal error: invalid pool pointer")
                    }
                }
            };
            gc_box.data.borrow_mut().reset();
            gc_box.guard_count.set(1); // Start with guard_count = 1 for the allocating guard
            gc_box.ref_count.set(0);
            gc_box.marked.set(false);
            gc_box.pooled.set(false);

            // Assign new ID for reused object
            let id = self.next_object_id;
            self.next_object_id += 1;
            gc_box.id = id;

            self.id_to_ptr.insert(id, ptr);

            (id, ptr)
        } else {
            // Need to allocate new - check if current chunk has space
            let need_new_chunk = self
                .chunks
                .last()
                .is_none_or(|chunk| chunk.len() >= self.chunk_capacity);

            if need_new_chunk {
                // Create new chunk with fixed capacity
                self.chunks.push(Vec::with_capacity(self.chunk_capacity));
            }

            // Get the last chunk (guaranteed to exist and have space now)
            let chunk = match self.chunks.last_mut() {
                Some(c) => c,
                None => {
                    #[allow(clippy::panic)]
                    {
                        panic!("GC internal error: no chunk after creation")
                    }
                }
            };

            let id = self.next_object_id;
            self.next_object_id += 1;

            // Push the new object into the chunk
            chunk.push(GcBox::new(id, T::default()));

            // Get pointer to the just-pushed object
            // Safety: We just pushed, so last() is valid and chunk won't reallocate
            // (we pre-allocated with_capacity)
            let gc_box = match chunk.last() {
                Some(b) => b,
                None => {
                    #[allow(clippy::panic)]
                    {
                        panic!("GC internal error: chunk empty after push")
                    }
                }
            };
            gc_box.guard_count.set(1); // Start with guard_count = 1 for the allocating guard
            let ptr = NonNull::from(gc_box);

            self.id_to_ptr.insert(id, ptr);

            (id, ptr)
        };

        // Track allocations since last GC
        self.net_allocs += 1;
        // Trigger GC when net allocations since last GC exceeds threshold
        if self.gc_threshold > 0 && self.net_allocs >= self.gc_threshold {
            self.collect();
        }

        Gc {
            id,
            ptr,
            _marker: std::marker::PhantomData,
        }
    }

    /// Increment guard_count for an object (used by guard)
    fn inc_guard(&mut self, object_id: usize) {
        let Some(&ptr) = self.id_to_ptr.get(&object_id) else {
            return;
        };
        // Safety: ptr from id_to_ptr is valid
        let gc_box = unsafe { ptr.as_ref() };
        if gc_box.pooled.get() {
            return;
        }
        gc_box.guard_count.set(gc_box.guard_count.get() + 1);
    }

    /// Decrement guard_count for an object, pool if both counts reach 0
    fn dec_guard(&mut self, object_id: usize) {
        let Some(&ptr) = self.id_to_ptr.get(&object_id) else {
            return;
        };
        // Safety: ptr from id_to_ptr is valid
        let gc_box = unsafe { ptr.as_ref() };
        if gc_box.pooled.get() {
            return;
        }

        let count = gc_box.guard_count.get();
        if count > 0 {
            gc_box.guard_count.set(count - 1);
            if count == 1 && gc_box.ref_count.get() == 0 {
                // Both guard_count and ref_count are 0, pool immediately
                self.pool_object(object_id, ptr);
            }
        }
    }

    /// Increment ref_count for an object (used by ownership)
    fn inc_ref(&mut self, object_id: usize) {
        let Some(&ptr) = self.id_to_ptr.get(&object_id) else {
            return;
        };
        // Safety: ptr from id_to_ptr is valid
        let gc_box = unsafe { ptr.as_ref() };
        if gc_box.pooled.get() {
            return;
        }
        gc_box.ref_count.set(gc_box.ref_count.get() + 1);
    }

    /// Decrement ref_count for an object, pool if both counts reach 0
    fn dec_ref(&mut self, object_id: usize) {
        let Some(&ptr) = self.id_to_ptr.get(&object_id) else {
            return;
        };
        // Safety: ptr from id_to_ptr is valid
        let gc_box = unsafe { ptr.as_ref() };
        if gc_box.pooled.get() {
            return;
        }

        let count = gc_box.ref_count.get();
        if count > 0 {
            gc_box.ref_count.set(count - 1);
            if count == 1 && gc_box.guard_count.get() == 0 {
                // Both guard_count and ref_count are 0, pool immediately
                self.pool_object(object_id, ptr);
            }
        }
    }

    /// Move an object to the pool (internal helper)
    fn pool_object(&mut self, object_id: usize, ptr: NonNull<GcBox<T>>) {
        // Remove from id_to_ptr
        self.id_to_ptr.remove(&object_id);

        // Decrement net allocations (object is being deallocated)
        self.net_allocs -= 1;

        // Safety: ptr is valid
        let gc_box = unsafe { ptr.as_ref() };

        // Remove all ownership edges FROM this object and decrement their ref_counts
        if let Some(owned_edges) = self.ownership_edges.remove(&object_id) {
            for (owned_id, _) in owned_edges {
                self.dec_ref(owned_id);
            }
        }

        // Note: We don't remove edges TO this object here - they will be
        // naturally cleaned up when those owners are collected, or ignored
        // during mark phase (pooled objects are skipped).

        // Mark as pooled and reset
        gc_box.pooled.set(true);
        gc_box.data.borrow_mut().reset();

        // Add pointer to pool for reuse
        self.pool.push(ptr);
    }

    /// Mark phase: mark all objects reachable from roots (guard_count > 0)
    fn mark(&mut self) {
        // Collect roots from id_to_ptr (all live objects with guard_count > 0)
        let mut stack: Vec<NonNull<GcBox<T>>> = Vec::with_capacity(64);
        for &ptr in self.id_to_ptr.values() {
            let gc_box = unsafe { ptr.as_ref() };
            if gc_box.guard_count.get() > 0 {
                stack.push(ptr);
            }
        }

        // Iterative mark traversal using pointers directly
        while let Some(ptr) = stack.pop() {
            // Safety: ptr came from our chunks which are stable
            let gc_box = unsafe { ptr.as_ref() };

            // Already marked - skip
            if gc_box.marked.get() {
                continue;
            }

            // Mark this object
            gc_box.marked.set(true);

            // Push owned objects onto stack - pointers are stored directly
            if let Some(owned_edges) = self.ownership_edges.get(&gc_box.id) {
                for &(_owned_id, owned_ptr) in owned_edges {
                    stack.push(owned_ptr);
                }
            }
        }
    }

    /// Sweep phase: collect all unmarked objects and clear marks
    /// Returns number of objects collected
    fn sweep(&mut self) -> usize {
        let mut to_collect = Vec::new();

        // Iterate over live objects only (id_to_ptr contains only non-pooled objects)
        for (&id, &ptr) in &self.id_to_ptr {
            let gc_box = unsafe { ptr.as_ref() };
            if gc_box.marked.get() {
                // Clear mark for next collection
                gc_box.marked.set(false);
            } else {
                // Unmarked = unreachable, collect it
                to_collect.push(id);
            }
        }

        let collected = to_collect.len();
        for object_id in to_collect {
            self.collect_object_by_id(object_id);
        }
        collected
    }

    /// Run mark-and-sweep collection
    fn collect(&mut self) {
        self.mark();
        let collected = self.sweep();
        // Only reset net_allocs if we collected something
        // This prevents frequent GC when cycles prevent collection
        if collected > 0 {
            self.net_allocs = 0;
        } else {
            // No objects collected - double the threshold temporarily
            // to avoid spinning on futile GC attempts
            self.gc_threshold = self.gc_threshold.saturating_mul(2);
        }
    }

    /// Force a collection (for testing or explicit cleanup)
    fn force_collect(&mut self) {
        self.collect();
    }

    /// Collect a single object by ID (move to pool) - used by sweep for cycles
    fn collect_object_by_id(&mut self, object_id: usize) {
        let Some(&ptr) = self.id_to_ptr.get(&object_id) else {
            return;
        };

        // Safety: ptr from id_to_ptr is valid
        let gc_box = unsafe { ptr.as_ref() };

        if gc_box.pooled.get() {
            return;
        }

        self.pool_object(object_id, ptr);
    }

    /// Establish ownership: owner owns owned (increments owned's ref_count)
    fn own(&mut self, owner_id: usize, owned_id: usize) {
        // Don't allow self-ownership
        if owner_id == owned_id {
            return;
        }

        // Get pointers and verify both objects exist and are alive
        let owner_ptr = match self.id_to_ptr.get(&owner_id) {
            Some(&ptr) => {
                let gc_box = unsafe { ptr.as_ref() };
                if gc_box.pooled.get() {
                    return;
                }
                ptr
            }
            None => return,
        };
        let owned_ptr = match self.id_to_ptr.get(&owned_id) {
            Some(&ptr) => {
                let gc_box = unsafe { ptr.as_ref() };
                if gc_box.pooled.get() {
                    return;
                }
                ptr
            }
            None => return,
        };

        // Silence unused warning - owner_ptr not needed for ownership tracking
        let _ = owner_ptr;

        // Check if this edge already exists (don't double-increment)
        let edges = self.ownership_edges.entry(owner_id).or_default();
        if !edges.iter().any(|(id, _)| *id == owned_id) {
            edges.push((owned_id, owned_ptr));
            // New edge - increment ref_count of owned object
            self.inc_ref(owned_id);
        }
    }

    /// Release ownership: owner no longer owns owned (decrements owned's ref_count)
    fn disown(&mut self, owner_id: usize, owned_id: usize) {
        let removed = if let Some(owned_vec) = self.ownership_edges.get_mut(&owner_id) {
            // Find and remove the owned_id (swap_remove is O(1))
            if let Some(pos) = owned_vec.iter().position(|(id, _)| *id == owned_id) {
                owned_vec.swap_remove(pos);
                if owned_vec.is_empty() {
                    self.ownership_edges.remove(&owner_id);
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        if removed {
            // Edge was removed - decrement ref_count (may pool if 0)
            self.dec_ref(owned_id);
        }
    }

    /// Get statistics
    fn stats(&self) -> GcStats {
        let ownership_edge_count: usize = self.ownership_edges.values().map(|s| s.len()).sum();
        let total_objects: usize = self.chunks.iter().map(|c| c.len()).sum();

        GcStats {
            total_objects,
            pooled_objects: self.pool.len(),
            live_objects: total_objects - self.pool.len(),
            ownership_edges: ownership_edge_count,
        }
    }

    /// Set the GC threshold (0 = disable automatic collection)
    fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold as isize;
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
    /// Create a new heap with default chunk capacity
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Space::new())),
        }
    }

    /// Create a new heap with specified chunk capacity
    pub fn with_chunk_capacity(chunk_capacity: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(Space::with_chunk_capacity(chunk_capacity))),
        }
    }

    /// Create a new guard (root)
    pub fn create_guard(&self) -> Guard<T> {
        Guard {
            guarded_objects: RefCell::new(Vec::new()),
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
/// Objects allocated through a guard have guard_count incremented.
/// Uses Vec instead of HashSet for faster append-only tracking.
pub struct Guard<T: Default + Reset> {
    /// Object IDs guarded by this guard (append-only, may have duplicates)
    guarded_objects: RefCell<Vec<usize>>,

    /// Weak reference back to space for allocation
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
        let result = space.borrow_mut().alloc_internal();
        // Track this object (guard_count already set to 1 by alloc_internal)
        self.guarded_objects.borrow_mut().push(result.id);
        result
    }

    /// Increment guard_count for an existing object.
    /// Makes the object a root for mark-and-sweep.
    pub fn guard(&self, obj: &Gc<T>) {
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().inc_guard(obj.id);
            self.guarded_objects.borrow_mut().push(obj.id);
        }
    }

    /// Decrement guard_count for an object.
    /// Object may be collected if guard_count and ref_count both reach 0.
    pub fn unguard(&self, obj: &Gc<T>) {
        if let Some(space) = self.space.upgrade() {
            space.borrow_mut().dec_guard(obj.id);
        }
        // Note: We don't remove from guarded_objects (it would be O(n))
        // The dec_guard already happened, so drop will just do an extra
        // dec_guard on a potentially pooled object (which is safe/no-op)
    }
}

impl<T: Default + Reset> Drop for Guard<T> {
    fn drop(&mut self) {
        // Decrement guard_count for all guarded objects
        if let Some(space) = self.space.upgrade() {
            let guarded = std::mem::take(self.guarded_objects.get_mut());
            let mut space_ref = space.borrow_mut();
            for object_id in guarded {
                space_ref.dec_guard(object_id);
            }
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
        f.debug_struct("Gc").field("id", &self.id).finish()
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
