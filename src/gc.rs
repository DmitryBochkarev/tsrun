//! A cycle-breaking garbage collector for Rc-based object graphs.
//!
//! This module provides a mark-and-sweep garbage collector that can detect and break
//! reference cycles in `Rc`-based object graphs. Users implement the [`Traceable`] trait
//! on their types to define how the GC should traverse and unlink references.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    mem,
    rc::{Rc, Weak},
};

/// A trait for types that can be traced by the garbage collector.
///
/// Types that can form reference cycles should implement this trait to allow
/// the GC to traverse their references and break cycles when needed.
pub trait Traceable: 'static {
    /// Visit all [`Gc`] references held by this object.
    ///
    /// The implementation should call `tracer.trace()` for each [`Gc`] pointer
    /// this object holds. This allows the GC to traverse the object graph and
    /// mark reachable objects.
    ///
    fn trace(&self, tracer: &mut Tracer<'_>);

    /// Unlink all [`Gc`] references held by this object.
    ///
    /// This method is called by the GC when breaking cycles. The implementation
    /// should set all [`Gc`] fields to `None` or clear collections containing
    /// [`Gc`] pointers.
    fn unlink(&mut self);
}

/// A tracer used during the mark phase of garbage collection.
///
/// This type is passed to [`Traceable::trace`] and should be used to report
/// all [`Gc`] references held by an object.
pub struct Tracer<'a> {
    callback: &'a mut dyn FnMut(usize),
}

impl Tracer<'_> {
    /// Trace a [`Gc`] reference.
    ///
    /// Call this for every [`Gc`] pointer your object holds.
    pub fn trace<T: Traceable>(&mut self, gc: &Gc<T>) {
        (self.callback)(gc.id());
    }
}

/// The internal storage for a GC-managed value.
struct GcData<T: Traceable> {
    id: usize,
    space: WeakSpace<T>,
    value: RefCell<T>,
}

impl<T: Traceable> GcData<T> {
    fn trace_object(&self, callback: &mut dyn FnMut(usize)) {
        let mut tracer = Tracer { callback };
        self.value.borrow().trace(&mut tracer);
    }

    fn unlink_object(&self) {
        if let Ok(mut val) = self.value.try_borrow_mut() {
            val.unlink();
        }
    }
}

impl<T: Traceable> Drop for GcData<T> {
    fn drop(&mut self) {
        self.space.free_object(self.id);
    }
}

/// A garbage-collected smart pointer.
///
/// `Gc<T>` provides shared ownership of a value of type `T`, similar to `Rc<T>`,
/// but with the ability to be traced and collected by a [`Space`] when it becomes
/// part of an unreachable cycle.
///
/// `Gc` pointers are created through [`Space::alloc`].
pub struct Gc<T: Traceable> {
    ptr: Rc<GcData<T>>,
}

impl<T: Traceable> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Gc {
            ptr: Rc::clone(&self.ptr),
        }
    }
}

impl<T: Traceable + std::fmt::Debug> std::fmt::Debug for Gc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gc(id={}, {:?})", self.ptr.id, &*self.ptr.value.borrow())
    }
}

impl<T: Traceable> Gc<T> {
    /// Borrow the contained value immutably.
    pub fn borrow(&self) -> std::cell::Ref<'_, T> {
        self.ptr.value.borrow()
    }

    /// Borrow the contained value mutably.
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, T> {
        self.ptr.value.borrow_mut()
    }

    /// Get the unique ID of this GC object within its space.
    pub fn id(&self) -> usize {
        self.ptr.id
    }

    /// Get the strong reference count.
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.ptr)
    }

    /// Check if two Gc pointers point to the same object.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Rc::ptr_eq(&this.ptr, &other.ptr)
    }
}

struct WeakSpace<T: Traceable> {
    internal: Weak<RefCell<SpaceInternal<T>>>,
}

impl<T: Traceable> WeakSpace<T> {
    fn free_object(&self, id: usize) {
        if let Some(internal) = self.internal.upgrade() {
            if let Ok(mut internal) = internal.try_borrow_mut() {
                internal.free_object(id);
            }
        }
    }
}

struct SpaceInternal<T: Traceable> {
    roots: HashMap<usize, Rc<GcData<T>>>,
    objects: Vec<Option<Weak<GcData<T>>>>,
    free_list: Vec<usize>,
    marked: HashSet<usize>,
    /// Number of allocations since last GC
    allocs_since_gc: usize,
    /// Threshold for triggering GC (0 = disabled, only trigger when free_list empty)
    gc_threshold: usize,
}

/// A memory space that manages garbage-collected objects of type `T`.
///
/// `Space` is the central manager for GC objects. It tracks all allocated objects,
/// maintains a set of root objects, and performs mark-and-sweep collection to
/// identify and break unreachable cycles.
pub struct Space<T: Traceable> {
    internal: Rc<RefCell<SpaceInternal<T>>>,
}

/// Default GC threshold - collect after this many allocations
/// Set to 1024 to balance collection frequency vs overhead
pub const DEFAULT_GC_THRESHOLD: usize = 1024;

impl<T: Traceable> Space<T> {
    /// Create a new space with default capacity (1024 slots) and default GC threshold.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new space with the specified initial capacity and default GC threshold.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_threshold(capacity, DEFAULT_GC_THRESHOLD)
    }

    /// Create a new space with specified capacity and GC threshold.
    ///
    /// The GC threshold controls how often garbage collection runs:
    /// - `0`: Only collect when free_list is exhausted (lazy collection)
    /// - `n > 0`: Collect after every `n` allocations (eager collection)
    ///
    /// Lower thresholds reduce peak memory but increase GC overhead.
    /// Higher thresholds improve throughput but may use more memory.
    pub fn with_capacity_and_threshold(capacity: usize, gc_threshold: usize) -> Self {
        let internal = SpaceInternal::new(capacity, gc_threshold);
        Space {
            internal: Rc::new(RefCell::new(internal)),
        }
    }

    /// Allocate a new GC-managed object in this space.
    ///
    /// Returns a [`Gc`] smart pointer to the allocated object.
    pub fn alloc(&mut self, value: T) -> Gc<T> {
        let id = {
            let mut internal = self.internal.borrow_mut();
            internal.prepare_for_alloc();
            internal.allocate_id()
        };

        debug_assert!(
            id < self.internal.borrow().objects.len(),
            "Allocated id {} is out of bounds (objects.len() = {})",
            id,
            self.internal.borrow().objects.len()
        );
        debug_assert!(
            self.internal
                .borrow()
                .objects
                .get(id)
                .is_some_and(|slot| slot.is_none()),
            "Slot {} should be empty before allocation",
            id
        );

        let gc_data: Rc<GcData<T>> = Rc::new(GcData {
            id,
            space: WeakSpace {
                internal: Rc::downgrade(&self.internal),
            },
            value: RefCell::new(value),
        });

        if let Some(slot) = self.internal.borrow_mut().objects.get_mut(id) {
            *slot = Some(Rc::downgrade(&gc_data));
        }

        debug_assert!(
            self.internal
                .borrow()
                .objects
                .get(id)
                .and_then(|slot| slot.as_ref())
                .and_then(|weak| weak.upgrade())
                .is_some(),
            "Newly allocated object should be alive"
        );

        Gc { ptr: gc_data }
    }

    /// Add an object as a root, preventing it and everything reachable from it
    /// from being collected.
    ///
    /// Adding the same object multiple times has no effect (roots are stored in a set).
    pub fn add_root(&mut self, gc: &Gc<T>) {
        let id = gc.id();
        debug_assert!(
            self.internal
                .borrow()
                .objects
                .get(id)
                .and_then(|slot| slot.as_ref())
                .and_then(|w| w.upgrade())
                .is_some(),
            "Cannot add dead object {} as root",
            id
        );
        debug_assert_eq!(
            self.internal
                .borrow()
                .objects
                .get(id)
                .and_then(|slot| slot.as_ref())
                .and_then(|w| w.upgrade())
                .map(|o| o.id),
            Some(id),
            "Object id doesn't match slot {}",
            id,
        );
        self.internal
            .borrow_mut()
            .roots
            .insert(id, Rc::clone(&gc.ptr));
    }

    /// Remove an object from the roots.
    ///
    /// Returns `true` if the object was found and removed.
    pub fn remove_root(&mut self, gc: &Gc<T>) -> bool {
        self.internal.borrow_mut().roots.remove(&gc.id()).is_some()
    }

    /// Remove all roots.
    ///
    /// After calling this, all objects become eligible for collection
    /// unless they are held by local `Gc` references.
    pub fn clear_roots(&mut self) {
        self.internal.borrow_mut().roots.clear();
    }

    /// Run garbage collection.
    ///
    /// This performs a mark-and-sweep collection:
    /// 1. Mark all objects reachable from roots
    /// 2. For unmarked objects, call `unlink()` to break cycles
    /// 3. Reclaim memory from unreachable objects
    pub fn collect(&mut self) {
        self.internal.borrow_mut().collect();
    }

    /// Returns the number of tracked object slots (including dead weak refs).
    pub fn tracked_count(&self) -> usize {
        self.internal
            .borrow()
            .objects
            .iter()
            .filter(|o| o.is_some())
            .count()
    }

    /// Returns the number of available slots in the free list.
    pub fn free_count(&self) -> usize {
        self.internal.borrow().free_list.len()
    }

    /// Returns the number of objects that are still alive.
    pub fn alive_count(&self) -> usize {
        self.internal
            .borrow()
            .objects
            .iter()
            .filter(|o| o.as_ref().is_some_and(|w| w.upgrade().is_some()))
            .count()
    }

    /// Returns the number of root objects.
    pub fn roots_count(&self) -> usize {
        self.internal.borrow().roots.len()
    }

    /// Get the current GC threshold.
    ///
    /// Returns the number of allocations that trigger a collection.
    /// A value of 0 means threshold-based collection is disabled.
    pub fn gc_threshold(&self) -> usize {
        self.internal.borrow().gc_threshold
    }

    /// Set the GC threshold.
    ///
    /// - `0`: Disable threshold-based collection (only collect when free_list exhausted)
    /// - `n > 0`: Collect after every `n` allocations
    ///
    /// Lower values reduce peak memory usage but increase GC overhead.
    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.internal.borrow_mut().gc_threshold = threshold;
    }

    /// Returns the number of allocations since the last GC.
    pub fn allocs_since_gc(&self) -> usize {
        self.internal.borrow().allocs_since_gc
    }
}

impl<T: Traceable> Default for Space<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Traceable> Drop for Space<T> {
    fn drop(&mut self) {
        self.internal.borrow_mut().collect();
    }
}

impl<T: Traceable> SpaceInternal<T> {
    fn new(capacity: usize, gc_threshold: usize) -> Self {
        SpaceInternal {
            roots: HashMap::new(),
            objects: (0..capacity).map(|_| None).collect(),
            free_list: (0..capacity).rev().collect(),
            marked: HashSet::new(),
            allocs_since_gc: 0,
            gc_threshold,
        }
    }

    fn prepare_for_alloc(&mut self) {
        // Trigger GC if threshold exceeded (and threshold is enabled)
        if self.gc_threshold > 0 && self.allocs_since_gc >= self.gc_threshold {
            self.collect();
        }
        // Also trigger if free_list is empty (must have space to allocate)
        if self.free_list.is_empty() {
            self.collect();
        }
        // Track allocation
        self.allocs_since_gc += 1;
    }

    fn allocate_id(&mut self) -> usize {
        match self.free_list.pop() {
            Some(id) => {
                debug_assert!(
                    id < self.objects.len(),
                    "Free list contained invalid id {} (objects.len() = {})",
                    id,
                    self.objects.len()
                );
                debug_assert!(
                    self.objects.get(id).is_some_and(|slot| slot.is_none()),
                    "Slot {} from free list should be empty",
                    id
                );
                id
            }
            None => {
                let old_len = self.objects.len();
                let reserve = old_len;
                let id = old_len;
                self.objects.extend((0..=reserve).map(|_| None));
                self.free_list
                    .extend(((old_len + 1)..(old_len + 1 + reserve)).rev());

                debug_assert_eq!(
                    self.objects.len(),
                    old_len + reserve + 1,
                    "Objects vector should have grown"
                );
                debug_assert!(
                    id < self.objects.len(),
                    "Newly allocated id {} should be valid",
                    id
                );

                id
            }
        }
    }

    fn free_object(&mut self, id: usize) {
        debug_assert!(
            id < self.objects.len(),
            "Cannot free invalid id {} (objects.len() = {})",
            id,
            self.objects.len()
        );

        if let Some(slot) = self.objects.get_mut(id) {
            if slot.is_some() {
                *slot = None;
                debug_assert!(
                    !self.free_list.contains(&id),
                    "Double-free detected: id {} is already in free list",
                    id
                );
                self.free_list.push(id);
            }
        }
    }

    fn collect(&mut self) {
        self.mark_reachable();
        self.break_cycles();
        // Reset allocation counter after collection
        self.allocs_since_gc = 0;
    }

    fn mark_reachable(&mut self) {
        self.marked.clear();

        let roots = mem::take(&mut self.roots);

        // Mark from each root
        for id in roots.keys() {
            self.mark_from(*id);
        }

        self.roots = roots;
    }

    fn mark_from(&mut self, id: usize) {
        if !self.marked.insert(id) {
            return; // Already marked
        }

        // Get strong reference to object so we can trace it
        let Some(obj) = self
            .objects
            .get(id)
            .and_then(|slot| slot.as_ref())
            .and_then(|w| w.upgrade())
        else {
            // Object is dead, free the slot now
            if let Some(slot) = self.objects.get_mut(id) {
                *slot = None;
                debug_assert!(
                    !self.free_list.contains(&id),
                    "Dead object slot {} should not be in free list",
                    id
                );
                self.free_list.push(id);
            }
            return;
        };

        // Trace children and mark them recursively
        obj.trace_object(&mut |child_id| {
            self.mark_from(child_id);
        });
    }

    fn break_cycles(&mut self) {
        if self.marked.len() == self.objects.len() {
            return;
        }

        for (id, slot) in self.objects.iter_mut().enumerate() {
            if self.marked.contains(&id) {
                debug_assert!(slot.is_some(), "Marked slot {} should not be empty", id);
                debug_assert!(
                    !self.free_list.contains(&id),
                    "Marked slot {} should not be in free list",
                    id
                );
                continue;
            }

            if let Some(weak) = slot {
                if let Some(obj) = weak.upgrade() {
                    debug_assert_eq!(obj.id, id, "Object id {} doesn't match slot {}", obj.id, id);
                    // Only unlink if no external references exist (strong_count == 1 means
                    // only this temporary `obj` reference exists, plus weak refs in space)
                    // If strong_count > 1, external code still holds references to this object
                    // so we should NOT clear its properties
                    if Rc::strong_count(&obj) == 1 {
                        obj.unlink_object();
                    }
                }
                *slot = None;
                debug_assert!(
                    !self.free_list.contains(&id),
                    "Slot {} already in free list before adding",
                    id
                );
                self.free_list.push(id);
            }
        }

        // Debug assertions to verify invariants after cleanup
        #[cfg(debug_assertions)]
        self.verify_invariants();
    }

    #[cfg(debug_assertions)]
    fn verify_invariants(&self) {
        // Verify all live objects have correct ids
        for (id, slot) in self.objects.iter().enumerate() {
            if let Some(weak) = slot {
                if let Some(obj) = weak.upgrade() {
                    debug_assert_eq!(
                        obj.id, id,
                        "Object at slot {} has mismatched id {}",
                        id, obj.id
                    );
                }
            }
        }

        // Verify all live objects are marked (reachable)
        for (id, slot) in self.objects.iter().enumerate() {
            if let Some(weak) = slot {
                if weak.upgrade().is_some() {
                    debug_assert!(
                        self.marked.contains(&id),
                        "Live object {} should be marked as reachable",
                        id
                    );
                }
            }
        }

        // Verify all empty slots are in free_list
        for (id, slot) in self.objects.iter().enumerate() {
            if slot.is_none() {
                debug_assert!(
                    self.free_list.contains(&id),
                    "Empty slot {} should be in free_list",
                    id
                );
            }
        }

        // Verify free list contains only empty slots
        for &id in &self.free_list {
            debug_assert!(
                self.objects.get(id).is_some_and(|slot| slot.is_none()),
                "Free list contains non-empty slot {}",
                id
            );
        }

        // Verify all roots are marked
        for &id in self.roots.keys() {
            debug_assert!(self.marked.contains(&id), "Root {} should be marked", id);
        }
    }
}

/// A wrapper for simple values that don't contain GC references.
///
/// Use this when you want to store plain values in the GC without implementing
/// [`Traceable`] manually.
pub struct GcBox<T> {
    value: T,
}

impl<T: 'static> GcBox<T> {
    /// Create a new GcBox containing the given value.
    pub fn new(value: T) -> Self {
        GcBox { value }
    }

    /// Get a reference to the contained value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference to the contained value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Consume the box and return the contained value.
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: 'static> Traceable for GcBox<T> {
    fn trace(&self, _tracer: &mut Tracer<'_>) {
        // No GC references to trace
    }

    fn unlink(&mut self) {
        // No GC references to unlink
    }
}

impl<T> std::ops::Deref for GcBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for GcBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

/// Implement Traceable for Option<Gc<T>> for convenience.
impl<T: Traceable> Traceable for Option<Gc<T>> {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(gc) = self {
            tracer.trace(gc);
        }
    }

    fn unlink(&mut self) {
        *self = None;
    }
}

/// Implement Traceable for Vec<Gc<T>> for convenience.
impl<T: Traceable> Traceable for Vec<Gc<T>> {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for gc in self {
            tracer.trace(gc);
        }
    }

    fn unlink(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Node {
        value: i32,
        next: Option<Gc<Node>>,
    }

    impl Traceable for Node {
        fn trace(&self, tracer: &mut Tracer<'_>) {
            if let Some(next) = &self.next {
                tracer.trace(next);
            }
        }

        fn unlink(&mut self) {
            self.next = None;
        }
    }

    #[test]
    fn test_simple_allocation() {
        let mut space = Space::new();
        let node = space.alloc(Node {
            value: 42,
            next: None,
        });
        assert_eq!(node.borrow().value, 42);
    }

    #[test]
    fn test_cycle_cleanup() {
        let mut space = Space::with_capacity(8);

        {
            let node_a = space.alloc(Node {
                value: 1,
                next: None,
            });
            let node_b = space.alloc(Node {
                value: 2,
                next: None,
            });

            // Create cycle: A -> B -> A
            node_a.borrow_mut().next = Some(node_b.clone());
            node_b.borrow_mut().next = Some(node_a.clone());

            assert_eq!(space.alive_count(), 2);
        }

        // Nodes dropped but cycle keeps them alive
        assert_eq!(space.alive_count(), 2);

        space.collect();

        // Cycle broken, objects collected
        assert_eq!(space.alive_count(), 0);
    }

    #[test]
    fn test_rooted_objects_preserved() {
        let mut space = Space::with_capacity(8);

        let root = space.alloc(Node {
            value: 1,
            next: None,
        });
        let child = space.alloc(Node {
            value: 2,
            next: None,
        });

        root.borrow_mut().next = Some(child.clone());
        space.add_root(&root);

        drop(root);
        drop(child);

        space.collect();

        // Root and child should still be alive
        assert_eq!(space.alive_count(), 2);
    }

    #[test]
    fn test_gcbox_simple_value() {
        let mut space: Space<GcBox<i32>> = Space::new();
        let boxed = space.alloc(GcBox::new(42i32));
        assert_eq!(*boxed.borrow().get(), 42);
    }

    #[test]
    fn test_self_reference() {
        let mut space = Space::with_capacity(8);

        {
            let node = space.alloc(Node {
                value: 1,
                next: None,
            });
            node.borrow_mut().next = Some(node.clone());
            assert_eq!(space.alive_count(), 1);
        }

        // Self-reference keeps node alive
        assert_eq!(space.alive_count(), 1);

        space.collect();

        // Self-reference broken
        assert_eq!(space.alive_count(), 0);
    }

    #[test]
    fn test_remove_root() {
        let mut space = Space::with_capacity(8);

        let node = space.alloc(Node {
            value: 1,
            next: None,
        });
        space.add_root(&node);

        assert_eq!(space.roots_count(), 1);
        assert!(space.remove_root(&node));
        assert_eq!(space.roots_count(), 0);

        drop(node);
        space.collect();

        assert_eq!(space.alive_count(), 0);
    }

    #[test]
    fn test_mixed_scenario() {
        let mut space = Space::with_capacity(16);

        // Create a rooted chain: root -> A -> B
        let root = space.alloc(Node {
            value: 0,
            next: None,
        });
        let obj_a = space.alloc(Node {
            value: 1,
            next: None,
        });
        let obj_b = space.alloc(Node {
            value: 2,
            next: None,
        });

        root.borrow_mut().next = Some(obj_a.clone());
        obj_a.borrow_mut().next = Some(obj_b.clone());
        space.add_root(&root);

        // Create an unreachable cycle: X -> Y -> X
        let obj_x = space.alloc(Node {
            value: 3,
            next: None,
        });
        let obj_y = space.alloc(Node {
            value: 4,
            next: None,
        });
        obj_x.borrow_mut().next = Some(obj_y.clone());
        obj_y.borrow_mut().next = Some(obj_x.clone());

        // Drop local refs
        drop(root);
        drop(obj_a);
        drop(obj_b);
        drop(obj_x);
        drop(obj_y);

        assert_eq!(space.alive_count(), 5);

        space.collect();

        // Only rooted chain survives
        assert_eq!(space.alive_count(), 3);
    }

    struct MultiChild {
        children: Vec<Gc<MultiChild>>,
    }

    impl Traceable for MultiChild {
        fn trace(&self, tracer: &mut Tracer<'_>) {
            for child in &self.children {
                tracer.trace(child);
            }
        }

        fn unlink(&mut self) {
            self.children.clear();
        }
    }

    #[test]
    fn test_multiple_children() {
        let mut space = Space::with_capacity(16);

        let parent = space.alloc(MultiChild {
            children: Vec::new(),
        });
        let child1 = space.alloc(MultiChild {
            children: Vec::new(),
        });
        let child2 = space.alloc(MultiChild {
            children: Vec::new(),
        });
        let child3 = space.alloc(MultiChild {
            children: Vec::new(),
        });

        parent.borrow_mut().children.push(child1.clone());
        parent.borrow_mut().children.push(child2.clone());
        parent.borrow_mut().children.push(child3.clone());

        // Create cycle through children
        child3.borrow_mut().children.push(parent.clone());

        space.add_root(&parent);

        drop(parent);
        drop(child1);
        drop(child2);
        drop(child3);

        space.collect();

        // All 4 should survive (reachable from root)
        assert_eq!(space.alive_count(), 4);
    }

    #[test]
    fn test_gc_threshold_default() {
        let space: Space<Node> = Space::new();
        assert_eq!(space.gc_threshold(), DEFAULT_GC_THRESHOLD);
    }

    #[test]
    fn test_gc_threshold_custom() {
        let space: Space<Node> = Space::with_capacity_and_threshold(100, 50);
        assert_eq!(space.gc_threshold(), 50);
    }

    #[test]
    fn test_gc_threshold_disabled() {
        let mut space: Space<Node> = Space::with_capacity_and_threshold(100, 0);
        assert_eq!(space.gc_threshold(), 0);

        // Create cycles that stay alive via cyclic references
        let mut nodes = Vec::new();
        for i in 0..25 {
            let a = space.alloc(Node {
                value: i,
                next: None,
            });
            let b = space.alloc(Node {
                value: i + 100,
                next: None,
            });
            // Create cycle to keep them alive
            a.borrow_mut().next = Some(b.clone());
            b.borrow_mut().next = Some(a.clone());
            nodes.push(a);
            nodes.push(b);
        }

        // All 50 should still be alive (cycles keep them alive, no GC ran)
        assert_eq!(space.alive_count(), 50);

        // Drop local references - cycles still keep them alive
        drop(nodes);
        assert_eq!(space.alive_count(), 50);

        // Now collect - should break cycles
        space.collect();
        assert_eq!(space.alive_count(), 0);
    }

    #[test]
    fn test_gc_threshold_triggers_collection() {
        // Small threshold to trigger frequent GC
        let mut space: Space<Node> = Space::with_capacity_and_threshold(100, 10);

        // Create cycles that go out of scope
        for _ in 0..30 {
            let a = space.alloc(Node {
                value: 1,
                next: None,
            });
            let b = space.alloc(Node {
                value: 2,
                next: None,
            });
            // Create cycle
            a.borrow_mut().next = Some(b.clone());
            b.borrow_mut().next = Some(a.clone());
            // a, b go out of scope - become unreachable cycles
        }

        // With threshold=10, GC should have run multiple times
        // breaking cycles and keeping alive count low
        // Without threshold, we'd have 60 objects (30 * 2)
        // With threshold, cycles get collected periodically
        assert!(
            space.alive_count() < 30,
            "Threshold GC should keep alive count bounded: {}",
            space.alive_count()
        );
    }

    #[test]
    fn test_set_gc_threshold() {
        let mut space: Space<Node> = Space::new();
        assert_eq!(space.gc_threshold(), DEFAULT_GC_THRESHOLD);

        space.set_gc_threshold(256);
        assert_eq!(space.gc_threshold(), 256);

        space.set_gc_threshold(0);
        assert_eq!(space.gc_threshold(), 0);
    }

    #[test]
    fn test_allocs_since_gc() {
        let mut space: Space<Node> = Space::with_capacity_and_threshold(100, 0);
        assert_eq!(space.allocs_since_gc(), 0);

        // Allocate some objects
        for i in 0..5 {
            let _node = space.alloc(Node {
                value: i,
                next: None,
            });
        }
        assert_eq!(space.allocs_since_gc(), 5);

        // Manual collect should reset counter
        space.collect();
        assert_eq!(space.allocs_since_gc(), 0);
    }

    #[test]
    fn test_threshold_resets_after_collection() {
        let mut space: Space<Node> = Space::with_capacity_and_threshold(100, 5);

        // Allocate 5 objects to reach threshold
        for i in 0..5 {
            let _node = space.alloc(Node {
                value: i,
                next: None,
            });
        }
        // Counter is 5 (threshold reached but GC hasn't run yet)
        assert_eq!(space.allocs_since_gc(), 5);

        // 6th allocation: check finds counter >= threshold, runs GC, resets to 0, then increments
        let _node = space.alloc(Node {
            value: 99,
            next: None,
        });

        // Counter should be 1 (GC ran at start of alloc, reset to 0, then incremented)
        assert_eq!(space.allocs_since_gc(), 1);
    }

    #[test]
    fn test_gc_threshold_memory_bounded() {
        // This test verifies that threshold-based GC keeps memory bounded
        // when creating many cycles

        // With threshold = 20, GC runs every 20 allocations
        let mut space_with_threshold: Space<Node> = Space::with_capacity_and_threshold(1000, 20);

        // Without threshold (0), GC only runs when free_list exhausted
        let mut space_without: Space<Node> = Space::with_capacity_and_threshold(1000, 0);

        // Create 100 cycles (200 objects) in each space
        for _ in 0..100 {
            // With threshold
            {
                let a = space_with_threshold.alloc(Node {
                    value: 1,
                    next: None,
                });
                let b = space_with_threshold.alloc(Node {
                    value: 2,
                    next: None,
                });
                a.borrow_mut().next = Some(b.clone());
                b.borrow_mut().next = Some(a.clone());
            }

            // Without threshold
            {
                let a = space_without.alloc(Node {
                    value: 1,
                    next: None,
                });
                let b = space_without.alloc(Node {
                    value: 2,
                    next: None,
                });
                a.borrow_mut().next = Some(b.clone());
                b.borrow_mut().next = Some(a.clone());
            }
        }

        // Space with threshold should have far fewer live objects
        let alive_with = space_with_threshold.alive_count();
        let alive_without = space_without.alive_count();

        println!(
            "With threshold: {}, Without threshold: {}",
            alive_with, alive_without
        );

        // Without threshold, all 200 cycles should still be alive
        assert_eq!(alive_without, 200);

        // With threshold, most should have been collected
        assert!(
            alive_with < 50,
            "Threshold should keep alive count low: {}",
            alive_with
        );
    }
}
