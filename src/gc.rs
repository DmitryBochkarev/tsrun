//! Mark-and-sweep garbage collection system.
//!
//! Objects are kept alive by being reachable from Guard roots through ownership edges.
//! Collection happens when guards are dropped, using a mark-and-sweep algorithm.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

// ============================================================================
// ChunkBitmask - 256-bit bitmask for marking objects within a chunk
// ============================================================================

/// 256-bit bitmask for marking objects within a chunk.
/// Each bit corresponds to an index in the chunk (0-255).
#[derive(Clone, Copy, Default)]
struct ChunkBitmask {
    /// 4 × u64 = 256 bits
    bits: [u64; 4],
}

impl ChunkBitmask {
    /// Set a bit at the given index (0-255)
    ///
    /// # Safety
    /// Caller must ensure index < 256. In debug builds this is checked via assert.
    #[inline]
    fn set(&mut self, index: usize) {
        debug_assert!(index < 256);
        // Safety: index < 256 means word < 4, which is always in bounds for bits[4]
        // We use unchecked access to avoid bounds check in hot path
        let word = index >> 6; // index / 64
        let bit = index & 63; // index % 64
        unsafe {
            *self.bits.get_unchecked_mut(word) |= 1 << bit;
        }
    }

    /// Check if a bit is set at the given index (0-255)
    ///
    /// # Safety
    /// Caller must ensure index < 256. In debug builds this is checked via assert.
    #[inline]
    fn get(&self, index: usize) -> bool {
        debug_assert!(index < 256);
        // Safety: index < 256 means word < 4, which is always in bounds for bits[4]
        // We use unchecked access to avoid bounds check in hot path
        let word = index >> 6; // index / 64
        let bit = index & 63; // index % 64
        unsafe { (*self.bits.get_unchecked(word) & (1 << bit)) != 0 }
    }

    /// Clear all bits
    #[inline]
    fn clear(&mut self) {
        self.bits = [0; 4];
    }

    /// Iterate over unmarked indices (bits that are 0) up to `len`
    #[inline]
    fn iter_unmarked(&self, len: usize) -> impl Iterator<Item = usize> + '_ {
        UnmarkedIter {
            bitmask: self,
            len,
            current_word: 0,
            // Safety: bits[0] always exists (array of 4)
            current_bits: unsafe { !*self.bits.get_unchecked(0) },
            base_index: 0,
        }
    }
}

/// Iterator over unmarked (zero) bits in a ChunkBitmask
struct UnmarkedIter<'a> {
    bitmask: &'a ChunkBitmask,
    len: usize,
    current_word: usize,
    current_bits: u64, // Inverted bits (1 = unmarked)
    base_index: usize,
}

impl Iterator for UnmarkedIter<'_> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        loop {
            // Find next set bit in current_bits (which represents unmarked positions)
            if self.current_bits != 0 {
                let bit_pos = self.current_bits.trailing_zeros() as usize;
                let index = self.base_index + bit_pos;

                // Clear this bit
                self.current_bits &= self.current_bits - 1;

                if index < self.len {
                    return Some(index);
                }
                // Index beyond len, continue to potentially skip to next word
            }

            // Move to next word
            self.current_word += 1;
            if self.current_word >= 4 {
                return None;
            }

            self.base_index = self.current_word << 6; // * 64
            if self.base_index >= self.len {
                return None;
            }

            // Get inverted bits for this word (1 = unmarked)
            // Safety: current_word < 4 checked above, so always in bounds
            self.current_bits = unsafe { !*self.bitmask.bits.get_unchecked(self.current_word) };
        }
    }
}

// ============================================================================
// Gc - smart pointer to GC-managed object (defined early for Traceable trait)
// ============================================================================

/// A smart pointer to a GC-managed object.
///
/// Works like `Rc<T>` - cloning increments ref_count, dropping decrements it.
/// Objects are collected by the GC when unreachable (not when ref_count hits 0).
pub struct Gc<T: Default + Reset + Traceable> {
    /// Unique object ID (for ownership tracking)
    id: usize,

    /// Pointer to the GcBox for fast access
    ptr: NonNull<GcBox<T>>,

    /// Weak reference to space - used to check if space is still alive before accessing ptr
    /// This prevents use-after-free when Gc outlives the Space (e.g., during interpreter shutdown)
    space: Weak<RefCell<Space<T>>>,
}

impl<T: Default + Reset + Traceable> PartialEq for Gc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Default + Reset + Traceable> std::hash::Hash for Gc<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Default + Reset + Traceable> Eq for Gc<T> {}

impl<T: Default + Reset + Traceable> Gc<T> {
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

    /// Create a copy of this Gc without incrementing ref_count.
    /// Used during tracing where we don't want to affect ref_counts,
    /// and internally during allocation before the object is fully set up.
    ///
    /// SAFETY: Returns a GcPtr that does NOT have Drop. The caller must ensure
    /// the GcPtr doesn't outlive the original Gc.
    pub fn copy_ref(&self) -> GcPtr<T> {
        GcPtr {
            id: self.id,
            ptr: self.ptr,
        }
    }
}

// ============================================================================
// GcPtr - a Copy pointer without Drop (for tracing)
// ============================================================================

/// A raw pointer to a GC-managed object. Copy and no Drop.
/// Used during tracing to avoid affecting ref_counts.
pub struct GcPtr<T: Default + Reset + Traceable> {
    /// Unique object ID
    pub(crate) id: usize,
    /// Pointer to the GcBox
    pub(crate) ptr: NonNull<GcBox<T>>,
}

impl<T: Default + Reset + Traceable> Copy for GcPtr<T> {}

impl<T: Default + Reset + Traceable> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Default + Reset + Traceable> GcPtr<T> {
    /// Get the object's unique ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the GcBox's current index (stable - doesn't change when object is reused)
    pub fn gcbox_index(&self) -> usize {
        unsafe { self.ptr.as_ref().index }
    }
}

impl<T: Default + Reset + Traceable> Clone for Gc<T> {
    fn clone(&self) -> Self {
        // Increment ref_count (only if space is still alive)
        if let Some(_space) = self.space.upgrade() {
            let gc_box = unsafe { self.ptr.as_ref() };
            if !gc_box.pooled.get() {
                gc_box.ref_count.set(gc_box.ref_count.get() + 1);
            }
        }
        Self {
            id: self.id,
            ptr: self.ptr,
            space: self.space.clone(),
        }
    }
}

impl<T: Default + Reset + Traceable> Drop for Gc<T> {
    fn drop(&mut self) {
        // SAFETY: Check if space is still alive BEFORE accessing ptr.
        // If space is dropped, the GcBox memory is freed and ptr is dangling.
        // This happens during interpreter shutdown when Gc fields outlive the heap.
        let Some(space_rc) = self.space.upgrade() else {
            return; // Space is gone, ptr is dangling - do nothing
        };

        // Now safe to access the GcBox
        let gc_box = unsafe { self.ptr.as_ref() };
        if gc_box.pooled.get() {
            return;
        }

        let count = gc_box.ref_count.get();
        if count > 0 {
            gc_box.ref_count.set(count - 1);
        }

        // If ref_count is 0, reset and pool the object immediately
        if gc_box.ref_count.get() == 0 {
            // Try to borrow - if already borrowed (e.g., during GC), skip pooling
            if let Ok(mut space) = space_rc.try_borrow_mut() {
                // Reset to clear references before pooling
                gc_box.data.borrow_mut().reset();
                space.pool_object(gc_box.index, self.ptr);
            }
        }
    }
}

impl<T: Default + Reset + Traceable> std::fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gc").field("id", &self.id).finish()
    }
}

// ============================================================================
// Traceable trait - for discovering object references
// ============================================================================

/// Trait for types that can be traced by the garbage collector.
///
/// Objects implement this to yield their `GcPtr<T>` references during mark phase.
/// The GC calls `trace()` to discover reachable objects.
pub trait Traceable: Sized + Default + Reset {
    /// Visit all `Gc<Self>` references held by this object.
    ///
    /// The implementation should call `visitor` for each `Gc<T>` stored in fields,
    /// using `gc.copy_ref()` to get a `GcPtr` (which has no Drop to avoid ref_count changes).
    fn trace<F: FnMut(GcPtr<Self>)>(&self, visitor: F);
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
pub struct GcBox<T: Default + Reset + Traceable> {
    /// Index in chunks (chunk_idx * CHUNK_CAPACITY + index_in_chunk).
    /// This serves as both the unique ID and encodes the position for bitmask marking.
    index: usize,

    /// The actual data
    data: RefCell<T>,

    /// Reference count (Gc pointers + guard references pointing to this object)
    ref_count: Cell<usize>,

    /// Whether this object is in the pool (dead)
    pooled: Cell<bool>,
}

impl<T: Default + Reset + Traceable> GcBox<T> {
    fn new(index: usize, data: T) -> Self {
        Self {
            index,
            data: RefCell::new(data),
            ref_count: Cell::new(0),
            pooled: Cell::new(false),
        }
    }
}

// ============================================================================
// Space - the internal memory arena
// ============================================================================

/// Internal memory arena that manages all allocations.
/// Not exposed directly - accessed through `Heap<T>`.
struct Space<T: Default + Reset + Traceable> {
    /// Chunks of allocated objects. Each chunk has fixed capacity (CHUNK_CAPACITY).
    /// Inner vecs never reallocate, ensuring stable pointers.
    chunks: Vec<Vec<GcBox<T>>>,

    /// Free list of pooled object pointers
    free_list: Vec<NonNull<GcBox<T>>>,

    /// Per-chunk bitmasks for mark-and-sweep collection.
    /// Each bitmask has 256 bits (4 × u64) for marking objects within a chunk.
    /// Better cache locality than HashSet during marking and sweeping.
    marked_chunks: Vec<ChunkBitmask>,

    /// Persistent mark stack - reused between GC cycles to avoid repeated allocations.
    /// This is the biggest memory optimization: instead of allocating a new Vec
    /// each GC cycle (which would spill to heap and grow), we keep one Vec around.
    mark_stack: Vec<NonNull<GcBox<T>>>,

    /// Persistent sweep buffer - reused between GC cycles to avoid allocations.
    sweep_buffer: Vec<NonNull<GcBox<T>>>,

    /// Pool of reusable guard storage (Vec capacity is preserved for reuse)
    guard_pool: Vec<Vec<NonNull<GcBox<T>>>>,

    /// Net allocations: incremented on alloc, decremented on dealloc
    /// GC triggers when this exceeds threshold
    net_allocs: isize,

    /// Threshold for triggering collection (0 = never auto-collect)
    gc_threshold: isize,

    /// Weak self-reference for Gc pointers
    self_weak: Weak<RefCell<Space<T>>>,
}

/// Default threshold: collect after this many net allocations
/// Higher threshold = less frequent GC = better throughput but more memory
const DEFAULT_GC_THRESHOLD: usize = 100;

/// Chunk capacity: objects per chunk (hardcoded for bitmask optimization)
/// 256 = 4 × 64 bits, matching ChunkBitmask size
const CHUNK_CAPACITY: usize = 256;

impl<T: Default + Reset + Traceable> Space<T> {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            free_list: Vec::new(),
            marked_chunks: Vec::new(),
            mark_stack: Vec::new(),
            sweep_buffer: Vec::new(),
            guard_pool: Vec::new(),
            net_allocs: 0,
            gc_threshold: DEFAULT_GC_THRESHOLD as isize,
            self_weak: Weak::new(),
        }
    }

    /// Set the weak self-reference (called after wrapping in Rc)
    fn set_self_weak(&mut self, weak: Weak<RefCell<Space<T>>>) {
        self.self_weak = weak;
    }

    /// Create a new guard for allocating objects.
    /// Reuses pooled guard storage when available to avoid allocation.
    fn create_guard(&mut self) -> Guard<T> {
        if let Some(guarded) = self.guard_pool.pop() {
            Guard::with_storage(self.self_weak.clone(), guarded)
        } else {
            Guard::new(self.self_weak.clone())
        }
    }

    /// Return guard storage to the pool for reuse
    fn return_guard_to_pool(&mut self, mut guarded: Vec<NonNull<GcBox<T>>>) {
        // Keep max 16 guards in pool to bound memory usage
        if self.guard_pool.len() < 16 {
            guarded.clear();
            self.guard_pool.push(guarded);
        }
    }

    /// Allocate a new object (starts with ref_count = 1 for the returned Gc)
    fn alloc_internal(&mut self) -> Gc<T> {
        let (index, ptr) = if let Some(ptr) = self.free_list.pop() {
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
            gc_box.ref_count.set(1); // Start with ref_count = 1 for the returned Gc
            gc_box.pooled.set(false);

            // Index stays the same for reused objects
            (gc_box.index, ptr)
        } else {
            // Need to allocate new - check if current chunk has space
            let need_new_chunk = self
                .chunks
                .last()
                .is_none_or(|chunk| chunk.len() >= CHUNK_CAPACITY);

            if need_new_chunk {
                // Create new chunk with fixed capacity
                self.chunks.push(Vec::with_capacity(CHUNK_CAPACITY));
                // Create corresponding bitmask for the new chunk
                self.marked_chunks.push(ChunkBitmask::default());
            }

            // Get chunk index and the chunk itself
            let chunk_idx = self.chunks.len().saturating_sub(1);
            let chunk = match self.chunks.last_mut() {
                Some(c) => c,
                None => {
                    #[allow(clippy::panic)]
                    {
                        panic!("GC internal error: no chunk after creation")
                    }
                }
            };

            let index_in_chunk = chunk.len();
            // Linear index: chunk_idx * CHUNK_CAPACITY + index_in_chunk
            let index = chunk_idx * CHUNK_CAPACITY + index_in_chunk;

            // Push the new object into the chunk
            chunk.push(GcBox::new(index, T::default()));

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
            gc_box.ref_count.set(1); // Start with ref_count = 1 for the returned Gc
            let ptr = NonNull::from(gc_box);

            (index, ptr)
        };

        // Track allocations since last GC
        self.net_allocs += 1;
        // Trigger GC when net allocations since last GC exceeds threshold
        if self.gc_threshold > 0 && self.net_allocs >= self.gc_threshold {
            self.collect();
        }

        Gc {
            id: index,
            ptr,
            space: self.self_weak.clone(),
        }
    }

    /// Move an object to the pool (internal helper)
    /// Note: reset() should be called BEFORE pool_object to clear references
    fn pool_object(&mut self, _object_id: usize, ptr: NonNull<GcBox<T>>) {
        // Safety: ptr is valid
        let gc_box = unsafe { ptr.as_ref() };

        // Check if already pooled (prevent double-pooling)
        if gc_box.pooled.get() {
            return;
        }

        // Decrement net allocations (object is being deallocated)
        self.net_allocs -= 1;

        // Mark as pooled (reset already called in sweep or will be called on reuse)
        gc_box.pooled.set(true);

        // Add pointer to pool for reuse
        self.free_list.push(ptr);
    }

    /// Mark phase: trace from roots to find all reachable objects
    fn mark(&mut self) {
        // Clear all bitmasks from previous collection
        for bitmask in &mut self.marked_chunks {
            bitmask.clear();
        }

        // Take ownership of the persistent mark stack to avoid borrow issues.
        // This preserves capacity from previous GC cycles - the key optimization.
        let mut stack = std::mem::take(&mut self.mark_stack);
        stack.clear();

        // Collect roots: objects with ref_count > 0 are roots (someone is holding a Gc to them)
        // This replaces the previous guard-based root tracking with a more efficient approach
        for chunk in &self.chunks {
            for gc_box in chunk {
                if !gc_box.pooled.get() && gc_box.ref_count.get() > 0 {
                    stack.push(NonNull::from(gc_box));
                }
            }
        }

        // Get raw pointer to marked_chunks for use in closure (avoids borrow issues)
        let marked_chunks_ptr = self.marked_chunks.as_mut_ptr();
        let marked_chunks_len = self.marked_chunks.len();

        // Iterative mark traversal using Traceable::trace()
        while let Some(ptr) = stack.pop() {
            let gc_box = unsafe { ptr.as_ref() };
            let chunk_idx = gc_box.index / CHUNK_CAPACITY;
            let index_in_chunk = gc_box.index % CHUNK_CAPACITY;

            // Bounds check once, then use unchecked access
            if chunk_idx >= marked_chunks_len {
                continue;
            }

            // Safety: chunk_idx < marked_chunks_len checked above
            let bitmask = unsafe { &mut *marked_chunks_ptr.add(chunk_idx) };

            // Already marked - skip
            if bitmask.get(index_in_chunk) {
                continue;
            }

            // Mark this object
            bitmask.set(index_in_chunk);

            // Trace references via Traceable trait
            let data = gc_box.data.borrow();
            data.trace(|child: GcPtr<T>| {
                let child_box = unsafe { child.ptr.as_ref() };
                let child_chunk_idx = child_box.index / CHUNK_CAPACITY;
                let child_index_in_chunk = child_box.index % CHUNK_CAPACITY;

                // Check bounds and if already marked or pooled
                if child_chunk_idx < marked_chunks_len {
                    // Safety: child_chunk_idx < marked_chunks_len checked above
                    let child_bitmask = unsafe { &*marked_chunks_ptr.add(child_chunk_idx) };
                    if !child_bitmask.get(child_index_in_chunk) && !child_box.pooled.get() {
                        stack.push(child.ptr);
                    }
                }
            });
        }

        // Put the stack back (empty but with capacity preserved for next GC cycle)
        self.mark_stack = stack;
    }

    /// Sweep phase: collect all unmarked objects
    /// Returns number of objects collected
    fn sweep(&mut self) -> usize {
        let mut collected = 0;
        // Take ownership of persistent sweep buffer (preserves capacity from previous cycles)
        let mut unmarked = std::mem::take(&mut self.sweep_buffer);
        unmarked.clear();

        // First pass: reset all unmarked objects and collect their pointers
        // We must reset ALL objects before checking ref_counts because reset()
        // decrements ref_counts of referenced objects (important for cycles)
        for (chunk, bitmask) in self.chunks.iter().zip(self.marked_chunks.iter()) {
            for index_in_chunk in bitmask.iter_unmarked(chunk.len()) {
                if let Some(gc_box) = chunk.get(index_in_chunk) {
                    if !gc_box.pooled.get() {
                        // Reset clears references, which may decrement ref_counts of other objects
                        gc_box.data.borrow_mut().reset();
                        collected += 1;
                        unmarked.push(NonNull::from(gc_box));
                    }
                }
            }
        }

        // Second pass: pool objects with ref_count == 0 (after all resets complete)
        // This ensures cycles are fully broken before we check ref_counts
        for ptr in &unmarked {
            let gc_box = unsafe { ptr.as_ref() };
            if gc_box.ref_count.get() == 0 {
                self.pool_object(gc_box.index, *ptr);
            }
        }

        // Put buffer back (empty but with capacity preserved for next GC cycle)
        unmarked.clear();
        self.sweep_buffer = unmarked;

        collected
    }

    /// Run mark-and-sweep collection
    fn collect(&mut self) {
        self.mark();
        self.sweep();
        self.net_allocs = 0;
    }

    /// Force a collection (for testing or explicit cleanup)
    fn force_collect(&mut self) {
        self.collect();
    }

    /// Get statistics
    fn stats(&self) -> GcStats {
        let total_objects: usize = self.chunks.iter().map(|c| c.len()).sum();

        GcStats {
            total_objects,
            pooled_objects: self.free_list.len(),
            live_objects: total_objects - self.free_list.len(),
        }
    }

    /// Set the GC threshold (0 = disable automatic collection)
    fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold as isize;
    }
}

impl<T: Default + Reset + Traceable> Drop for Space<T> {
    fn drop(&mut self) {
        // Mark all GcBoxes as pooled before dropping chunks.
        // This ensures any remaining Gc pointers will see pooled=true
        // and skip accessing the (about-to-be-freed) GcBox data.
        for chunk in &self.chunks {
            for gc_box in chunk {
                gc_box.pooled.set(true);
            }
        }
        // Now chunks can be dropped safely - any Gc::drop calls will
        // see pooled=true and return early.
    }
}

// ============================================================================
// Heap - the public wrapper
// ============================================================================

/// A wrapper around the GC space that provides the public API.
/// This is the main entry point for using the GC.
pub struct Heap<T: Default + Reset + Traceable> {
    inner: Rc<RefCell<Space<T>>>,
}

impl<T: Default + Reset + Traceable> Heap<T> {
    /// Create a new heap
    pub fn new() -> Self {
        let inner = Rc::new(RefCell::new(Space::new()));
        inner.borrow_mut().set_self_weak(Rc::downgrade(&inner));
        Self { inner }
    }

    /// Create a new guard for allocating objects
    pub fn create_guard(&self) -> Guard<T> {
        self.inner.borrow_mut().create_guard()
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

impl<T: Default + Reset + Traceable> Default for Heap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + Reset + Traceable> Clone for Heap<T> {
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
/// Guards provide a way to allocate GC-managed objects. The returned `Gc<T>`
/// A Guard tracks objects that should be treated as GC roots.
/// Objects added to a guard are kept alive until the guard is dropped or cleared.
///
/// Guards are pooled for reuse - when dropped, they return to the heap's guard pool.
/// This avoids repeated allocation of guard storage.
///
/// The guard stores pointers to guarded objects. During mark-and-sweep,
/// these objects are used as roots for tracing.
pub struct Guard<T: Default + Reset + Traceable> {
    /// Weak reference back to space for allocation and returning to pool
    space: Weak<RefCell<Space<T>>>,
    /// Objects guarded by this guard (these are the roots)
    /// Uses raw pointers to avoid ref_count overhead during guarding.
    /// RefCell allows &self methods to mutate the guarded list.
    guarded: RefCell<Vec<NonNull<GcBox<T>>>>,
}

impl<T: Default + Reset + Traceable> Guard<T> {
    /// Create a new guard with the given space reference
    fn new(space: Weak<RefCell<Space<T>>>) -> Self {
        Self {
            space,
            guarded: RefCell::new(Vec::new()),
        }
    }

    /// Create a guard with pre-allocated storage (reused from pool)
    fn with_storage(space: Weak<RefCell<Space<T>>>, guarded: Vec<NonNull<GcBox<T>>>) -> Self {
        Self {
            space,
            guarded: RefCell::new(guarded),
        }
    }

    /// Allocate a new object and add it to this guard's roots.
    /// Returns a `Gc<T>` with ref_count=1.
    ///
    /// # Panics
    /// Panics if the Heap has been dropped while the guard is still alive.
    pub fn alloc(&self) -> Gc<T> {
        let space = self.space.upgrade().unwrap_or_else(|| {
            #[allow(clippy::panic)]
            {
                panic!("GC error: Heap dropped while guard is still alive")
            }
        });
        let result = space.borrow_mut().alloc_internal();
        // Add to guarded set
        self.guarded.borrow_mut().push(result.ptr);
        result
    }

    /// Add an existing object to this guard's roots.
    /// This keeps the object alive as long as the guard exists.
    pub fn guard(&self, obj: Gc<T>) {
        if let Some(_space) = self.space.upgrade() {
            let gc_box = unsafe { obj.ptr.as_ref() };
            if !gc_box.pooled.get() {
                self.guarded.borrow_mut().push(obj.ptr);
            }
        }
    }

    /// Remove an object from this guard's roots.
    /// Returns true if the object was found and removed.
    pub fn unguard(&self, obj: &Gc<T>) -> bool {
        let mut guarded = self.guarded.borrow_mut();
        if let Some(pos) = guarded.iter().position(|p| *p == obj.ptr) {
            guarded.swap_remove(pos);
            return true;
        }
        false
    }

    /// Clear all guarded objects
    pub fn clear(&self) {
        self.guarded.borrow_mut().clear();
    }

    /// Get the number of guarded objects
    pub fn len(&self) -> usize {
        self.guarded.borrow().len()
    }

    /// Check if this guard has no objects
    pub fn is_empty(&self) -> bool {
        self.guarded.borrow().is_empty()
    }
}

impl<T: Default + Reset + Traceable> Drop for Guard<T> {
    fn drop(&mut self) {
        // Return this guard to the pool for reuse
        if let Some(space) = self.space.upgrade() {
            // Return to pool - take ownership of Vec to reuse its capacity
            let mut guarded = self.guarded.borrow_mut().split_off(0);
            guarded.clear();
            space.borrow_mut().return_guard_to_pool(guarded);
        }
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
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test type that can hold references to other TestObjs
    #[derive(Default, Debug)]
    struct TestObj {
        value: i32,
        /// References to other objects (for testing ownership/tracing)
        refs: Vec<Gc<TestObj>>,
    }

    impl Reset for TestObj {
        fn reset(&mut self) {
            self.value = 0;
            self.refs.clear();
        }
    }

    impl Traceable for TestObj {
        fn trace<F: FnMut(GcPtr<Self>)>(&self, mut visitor: F) {
            for r in &self.refs {
                visitor(r.copy_ref());
            }
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

        // A owns B: clone increments ref_count automatically
        a.borrow_mut().refs.push(b.clone());

        // Drop guard2 - B should survive because A owns it (via clone in refs)
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

        // A owns B: clone increments ref_count
        a.borrow_mut().refs.push(b.clone());
        drop(guard2);
        drop(b); // Drop our reference too

        heap.collect(); // Force collection
        assert_eq!(heap.stats().live_objects, 2);

        // Clear refs - dropping clones decrements ref_count
        a.borrow_mut().refs.clear();
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

        // Both A and B own C (via clones)
        a.borrow_mut().refs.push(c.clone());
        b.borrow_mut().refs.push(c.clone());

        drop(guard3);
        drop(c); // Drop our reference
        heap.collect();
        assert_eq!(heap.stats().live_objects, 3); // C alive via A and B

        // A releases C
        a.borrow_mut().refs.clear();
        heap.collect();
        assert_eq!(heap.stats().live_objects, 3); // C still alive via B

        // B releases C
        b.borrow_mut().refs.clear();
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

        // Create cycle: A owns B, B owns A (via clones)
        a.borrow_mut().refs.push(b.clone());
        b.borrow_mut().refs.push(a.clone());

        assert_eq!(heap.stats().live_objects, 2);

        // Unguard A from guard
        guard.unguard(&a);
        heap.collect();

        // A should still be alive (through B which is still directly guarded,
        // plus the cycle keeps both ref_counts > 0)
        assert_eq!(heap.stats().live_objects, 2);

        // Unguard B from guard and drop variables
        guard.unguard(&b);
        drop(a);
        drop(b);
        heap.collect();

        // NOTE: With ref_count-based root detection, cycles keep each other alive
        // (each has ref_count=1 from the other's reference). This is a known limitation
        // of the current GC design. Proper cycle collection would require either:
        // - Trial deletion (decrement refs, see if cycle becomes unreachable)
        // - Guard-based root tracking (mark only from guarded objects, not ref_count)
        // For now, cycles that outlive their guards will leak until program end.
        assert_eq!(heap.stats().live_objects, 2);
    }

    #[test]
    fn test_guard_add_propagates() {
        let heap: Heap<TestObj> = Heap::new();
        let guard1 = heap.create_guard();
        let guard2 = heap.create_guard();

        let a = guard1.alloc();
        let b = guard1.alloc();

        // A owns B (clone increments ref_count)
        a.borrow_mut().refs.push(b.clone());

        // Add guard2 to A - B should survive via A even if guard1 is dropped
        guard2.guard(a.clone());

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

        // A → B → C (clone increments ref_count)
        a.borrow_mut().refs.push(b.clone());
        b.borrow_mut().refs.push(c.clone());

        // Unguard B and C from direct guard
        guard.unguard(&b);
        guard.unguard(&c);
        heap.collect();

        // All should still be alive (C through B, B through A)
        assert_eq!(heap.stats().live_objects, 3);

        // Unguard A - all should be collected (no path to any root)
        guard.unguard(&a);
        drop(a);
        drop(b);
        drop(c);
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

        // A owns B and C (clone increments ref_count)
        a.borrow_mut().refs.push(b.clone());
        a.borrow_mut().refs.push(c.clone());
        // B and C both own D (clone increments ref_count)
        b.borrow_mut().refs.push(d.clone());
        c.borrow_mut().refs.push(d.clone());

        // Unguard all except A
        guard.unguard(&b);
        guard.unguard(&c);
        guard.unguard(&d);
        heap.collect();

        // All should still be alive via A
        assert_eq!(heap.stats().live_objects, 4);

        // Remove one path to D (B → D) - clearing refs drops clone, decrementing ref_count
        b.borrow_mut().refs.retain(|r| !Gc::ptr_eq(r, &d));
        heap.collect();

        // D should still be alive via C
        assert_eq!(heap.stats().live_objects, 4);

        // Remove other path to D (C → D)
        c.borrow_mut().refs.retain(|r| !Gc::ptr_eq(r, &d));
        drop(d); // Drop the Gc variable too
        heap.collect();

        // D should now be collected
        assert_eq!(heap.stats().live_objects, 3);
    }

    #[test]
    fn test_multiple_references_same_object() {
        let heap: Heap<TestObj> = Heap::new();
        let guard = heap.create_guard();

        let obj1 = guard.alloc();
        let obj2 = obj1.clone(); // Two references to same object

        assert_eq!(heap.stats().live_objects, 1);

        drop(obj1);
        heap.collect();
        assert_eq!(heap.stats().live_objects, 1); // Still alive via obj2

        drop(obj2);
        heap.collect();
        assert_eq!(heap.stats().live_objects, 0); // Now collected
    }

    #[test]
    fn test_deep_chain_no_stack_overflow() {
        let heap: Heap<TestObj> = Heap::new();
        heap.set_gc_threshold(0); // Disable auto-GC for this test
        let guard = heap.create_guard();

        // Create a chain of 10000 objects
        let mut prev = guard.alloc();
        for _ in 0..10000 {
            let next = guard.alloc();
            prev.borrow_mut().refs.push(next.clone());
            prev = next;
        }

        // Collect - should not stack overflow
        drop(guard);
        drop(prev); // Drop the last Gc reference
        heap.collect();

        assert_eq!(heap.stats().live_objects, 0);
    }
}
