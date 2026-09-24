extern crate alloc;

use alloc::boxed::Box;
use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;
use core::mem;
use core::ops::Index;

pub use hashbrown::DefaultHashBuilder;
use hashbrown::HashMap;
pub use hashbrown::TryReserveError;

mod detail;
use detail::{KeyRef, KeyRefBorrow, Node, NonNullNode, NullableNode};
mod iter;
pub use iter::{Drain, IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys, Values, ValuesMut};

#[cfg(feature = "serde")]
mod serde;

/// A hash map which preserves the order of insertion.
///
/// Backed by [`hashbrown`] and its default hashing algorithm, currently [`foldhash`]. The hashing
/// algorithm can be changed on a per-map basis with [`with_hasher`] and [`with_capacity_and_hasher`].
/// More details on hashing algorithms and why foldhash was selected in the [`hashbrown`] crate.
///
/// To switch to the same hasher as [`HashMap`] use [`RandomState`]. This will switch to SipHash, a
/// HashDos resistant algorithm but it may be slower in some cases.
///
/// Being backed by a HashMap, this container has all the same requirements on its keys as
/// [`HashMap`]. Specifically, Keys must implement [`Eq`] and [`Hash`]. You can typically derive
/// these for types with `#[derive(PartialEq, Eq, hash)]`. If you implement these yourself you must
/// ensure the following holds:
///
/// ```text
/// k1 == k2 -> hash(k1) == hash(k2)
/// ```
/// You are also responsible to ensure Keys do not mutate such that their hash can change while in
/// the map. For more information, check [`HashMap`].
///
/// This container has some additional memory overhead on top of its backing HashMap. [`OrderedHashMap`]
/// holds two pointers to maintain the linked list. Each Value inserted has the overhead of three pointers, one for the key
/// and two for the linked list properties.
///
/// [`foldhash`]: https://crates.io/crates/foldhash
/// [`hashbrown`]: https://crates.io/crates/hashbrown
/// [`with_hasher`]: Self::with_hasher
/// [`with_capacity_and_hasher`]: Self::with_capacity_and_hasher
/// [`HashMap`]: std::collections::HashMap
/// [`RandomState`]: std::collections::hash_map::RandomState
pub struct OrderedHashMap<K, V, S = DefaultHashBuilder> {
    base: HashMap<KeyRef<K>, NonNullNode<K, V>, S>,
    head: NullableNode<K, V>,
    tail: NullableNode<K, V>,
    marker: PhantomData<Box<Node<K, V>>>,
}

impl<K, V, S> fmt::Debug for OrderedHashMap<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, S> Clone for OrderedHashMap<K, V, S>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        let mut clone = Self::with_capacity_and_hasher(self.base.len(), self.base.hasher().clone());
        clone.extend(self.iter().map(|(k, v)| (k.clone(), v.clone())));
        clone
    }
}

impl<K, V, S> Drop for OrderedHashMap<K, V, S> {
    fn drop(&mut self) {
        let mut head = self.head;
        while head.is_some() {
            if let Some(mut node) = head {
                // SAFETY: We assert NonNulls are always valid as an invariant, if this breaks it
                // is indicative of a bug at creation or usage of the collection.
                let node_ref = unsafe { node.as_mut() };
                let next_head = node_ref.next;
                // SAFETY: If the ref was valid to grab above, this is guaranteed safe.
                drop(unsafe { Box::from_raw(node_ref) });
                head = next_head;
            }
        }
    }
}

impl<K, V, S> Default for OrderedHashMap<K, V, S>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            base: Default::default(),
            head: None,
            tail: None,
            marker: PhantomData,
        }
    }
}

impl<K, V> OrderedHashMap<K, V, DefaultHashBuilder> {
    /// Create an empty map.
    ///
    /// Capacity is 0 so the map will not allocated until inserted into.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty map with at least the specified capacity.
    ///
    /// The map will allocate if capacity is nonzero.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            base: HashMap::with_capacity(capacity),
            ..Default::default()
        }
    }

    /// Returns a draining iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        let len = self.len();
        let (head, tail) = (self.head, self.tail);
        self.head = None;
        self.tail = None;
        self.base.clear(); // Empty map without dropping values
        Drain {
            head,
            tail,
            len,
            marker: PhantomData,
        }
    }
}

impl<K, V, S> OrderedHashMap<K, V, S> {
    /// Create an empty map which will use the given hasher.
    ///
    /// Capacity is 0 so the map will not allocate until inserted into.
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self {
            base: HashMap::with_hasher(hash_builder),
            head: None,
            tail: None,
            marker: PhantomData,
        }
    }

    /// Create an empty map with the given capacity and which will use the given hasher.
    ///
    /// The map will allocate if capacity is nonzero.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            base: HashMap::with_capacity_and_hasher(capacity, hash_builder),
            head: None,
            tail: None,
            marker: PhantomData,
        }
    }

    /// Returns a reference to the maps hasher.
    pub fn hasher(&self) -> &S {
        self.base.hasher()
    }

    /// The number of elements the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }

    /// The number of elements currently held in the map.
    pub fn len(&self) -> usize {
        self.base.len()
    }

    /// Returns whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// Removes all key-value pairs from the map.
    pub fn clear(&mut self) {
        self.base.clear();
        let mut head = self.head;
        while head.is_some() {
            if let Some(mut node) = head {
                // SAFETY: We assert NonNulls are always valid as an invariant, if this breaks it
                // is indicative of a bug at creation or usage of the collection.
                let node_ref = unsafe { node.as_mut() };
                let next_head = node_ref.next;
                // SAFETY: If the ref was valid to grab above, this is guaranteed safe.
                drop(unsafe { Box::from_raw(node_ref) });
                head = next_head;
            }
        }
        self.head = None;
        self.tail = None;
    }

    // TODO: retain()/drain_filter()

    /// Returns an iterator over the keys in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Returns an iterator over the keys in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys {
            inner: self.into_iter(),
        }
    }

    /// Returns an iterator over the values in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Returns an iterator over the values in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues {
            inner: self.into_iter(),
        }
    }

    /// Returns a mutable value iterator over the values in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.iter_mut(),
        }
    }

    /// Returns an iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.into_iter()
    }

    /// Returns a mutable value iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.into_iter()
    }

    // Detaches the node from the list. next and prev will point at each other
    // if they exist. Resets this node to point at None.
    #[inline]
    fn remove_node(&mut self, node: &mut NonNullNode<K, V>) {
        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let node_ref = unsafe { node.as_ref() };
        // if tail, update tail
        if self.tail == Some(*node) {
            self.tail = node_ref.prev;
        }
        // if head, update head
        if self.head == Some(*node) {
            self.head = node_ref.next;
        }

        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let prev_node = unsafe { node.as_mut() }.prev;
        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let next_node = unsafe { node.as_mut() }.next;
        if let Some(mut prev) = prev_node {
            // SAFETY: Accessing a NonNull is safe if it was created correctly.
            unsafe { prev.as_mut() }.next = next_node;
        }
        if let Some(mut next) = next_node {
            // SAFETY: Accessing a NonNull is safe if it was created correctly.
            unsafe { next.as_mut() }.prev = prev_node;
        }

        // Make this node point nowhere
        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let node_ref = unsafe { node.as_mut() };
        node_ref.next = None;
        node_ref.prev = None;
    }

    // Warning: Does not preserve any existing pointers. If this node was already
    // in the list you MUST call remove_node first.
    // TODO: Call remove_node at start? I think remove_node should be safe to call on a node with
    // no pointers and simply no-op
    #[inline]
    fn insert_head(&mut self, node: NonNullNode<K, V>) {
        // Moved into function, we can dd what we want
        let mut node = node;

        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let node_ref = unsafe { node.as_mut() };
        node_ref.next = self.head;
        if let Some(mut next) = node_ref.next {
            // SAFETY: Accessing a NonNull is safe if it was created correctly.
            unsafe { next.as_mut() }.prev = Some(node);
        }
        self.head = Some(node);
        if self.tail.is_none() {
            self.tail = Some(node);
        }
    }

    #[inline]
    fn insert_tail(&mut self, node: NonNullNode<K, V>) {
        // Moved into function, we can dd what we want
        let mut node = node;

        // SAFETY: Accessing a NonNull is safe if it was created correctly.
        let node_ref = unsafe { node.as_mut() };
        node_ref.prev = self.tail;
        if let Some(mut prev) = node_ref.prev {
            // SAFETY: Accessing a NonNull is safe if it was created correctly.
            unsafe { prev.as_mut() }.next = Some(node);
        }
        self.tail = Some(node);
        if self.head.is_none() {
            self.head = Some(node);
        }
    }
}

impl<K, V, S> OrderedHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Insert a new element into the map. If that key was associated with a value, returns the old
    /// element, otherwise None.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let key = Box::new(key);
        let (old_value, mut node) = if let Some(val) = self.base.get_mut(&KeyRef { key: &*key }) {
            // was in map
            // SAFETY: val was just fetched out of the map so it is safe as long as we guarantee it
            // is valid when inserted.
            let old = mem::replace(&mut (unsafe { val.as_mut() }).value, value);
            (Some(old), *val)
        } else {
            let node = Box::leak(Box::new(Node {
                key,
                value,
                next: None,
                prev: None,
            }))
            .into();
            (None, node)
        };

        if old_value.is_some() {
            // old value exists; update, don't insert
            self.remove_node(&mut node);
            self.insert_head(node);
        } else {
            // old value does not exist; insert and set head
            self.base.insert(
                KeyRef {
                    // SAFETY: Node was either just fetched or just created
                    key: &*unsafe { node.as_ref() }.key,
                },
                node,
            );
            self.insert_head(node);
        };
        old_value
    }

    /// Returns a reference to the value stored in the map.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base
            .get(KeyRefBorrow::from_ref(key))
            // SAFETY: Accessing a NonNull is safe it was created correctly.
            .map(|val| &unsafe { val.as_ref() }.value)
    }

    /// Returns a mutable reference to the value stored in the map.
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base
            .get_mut(KeyRefBorrow::from_ref(key))
            // SAFETY: Accessing a NonNull is safe it was created correctly.
            .map(|val| &mut unsafe { val.as_mut() }.value)
    }

    /// Returns a reference to the key and value stored in the map
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base.get(KeyRefBorrow::from_ref(key)).map(|val| {
            // SAFETY: Accessing a NonNull is safe it was created correctly.
            let val = unsafe { val.as_ref() };
            (&*val.key, &val.value)
        })
    }

    /// Returns a reference to key and mutable reference to value stored in the map
    #[inline]
    pub fn get_key_value_mut<Q>(&mut self, key: &Q) -> Option<(&K, &mut V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base.get_mut(KeyRefBorrow::from_ref(key)).map(|val| {
            // SAFETY: val was in the map - we maintain the invariant that items in the Hashmap are
            // not null
            let val = unsafe { val.as_mut() };
            (val.key.as_ref(), &mut val.value)
        })
    }

    /// Remove an element from the map, returning the stored value if there is one.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base
            .remove(KeyRefBorrow::from_ref(key))
            .map(|mut node| {
                self.remove_node(&mut node);
                node
            })
            // SAFETY: The box holding the data leaked it so making a new box will take ownership.
            // We return the value from it and drop the data after. Single free due to the original
            // box leaking it. No leaked memory, it is dropped when the box is at the end of the
            // scope.
            .map(|data| unsafe { Box::from_raw(data.as_ptr()) })
            .map(|data| data.value)
    }

    /// Remove an element from the map, returning the key and value if here is one.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base
            .remove(KeyRefBorrow::from_ref(key))
            .map(|mut node| {
                self.remove_node(&mut node);
                node
            })
            // SAFETY: The box holding the data leaked it so making a new box will take ownership.
            // We return the value from it and drop the data after. Single free due to the original
            // box leaking it. No leaked memory, it is dropped when the box is at the end of the
            // scope.
            .map(|data| unsafe { Box::from_raw(data.as_ptr()) }) // TODO: Box::into_inner when stable
            .map(|data| (*data.key, data.value))
    }

    /// Check if the map contains this key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.base.contains_key(KeyRefBorrow::from_ref(key))
    }

    /// Reserves capacity for at least `additional` more elements.
    ///
    /// # Panics
    /// Panics if the new size would overflow a usize.
    pub fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    ///
    /// # Errors
    /// Returns an error if the new size would overflow a usize.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.base.try_reserve(additional)
    }

    /// Shrinks the map as much as possible. May leave some space in accordance with the resize
    /// policy.
    pub fn shrink_to_fit(&mut self) {
        self.base.shrink_to_fit();
    }

    /// Shrinks the map down to a lower limit. May leave some space in accordance with the resize
    /// policy.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.base.shrink_to(min_capacity);
    }

    // TODO: entry()

    /// Removes and returns the most recently inserted key and value, if there is one.
    #[inline]
    pub fn pop_back_entry(&mut self) -> Option<(K, V)> {
        self.head.and_then(|head| {
            self.base
                // SAFETY: unsafe as_ref is only run if self.head is Some. This is safe if the linked
                // list is coherent, i.e. insert an remove are correct. We also guarantee the
                // KeyRefBorrow lifetime ends before remove_node() is called. If the lifetime did not
                // end first we would create a dangling pointer in KeyRefBorrow by moving values out of
                // the heap. This function *cannot* reuse the remove() function because that would
                // extend the KeyRefBorrow beyond the remove_node() call.
                .remove(KeyRefBorrow::from_ref(&*unsafe { head.as_ref() }.key))
                .map(|mut node| {
                    self.remove_node(&mut node);
                    node
                })
                // SAFETY: remove_node unlinks and removes the node, returning a valid object
                .map(|data| unsafe { Box::from_raw(data.as_ptr()) })
                .map(|data| (*data.key, data.value))
        })
    }

    /// Removes and returns the most recently inserted element, if there is one.
    #[inline]
    pub fn pop_back(&mut self) -> Option<V> {
        self.head.and_then(|head| {
            self.base
                // SAFETY: See pop_back_entry() note
                .remove(KeyRefBorrow::from_ref(&*unsafe { head.as_ref() }.key))
                .map(|mut node| {
                    self.remove_node(&mut node);
                    node
                })
                // SAFETY: remove_node unlinks and removes the node, returning a valid object
                .map(|data| unsafe { Box::from_raw(data.as_ptr()) })
                .map(|data| (data.value))
        })
    }

    /// Get the key and value from the element at the back, if there is one
    #[inline]
    pub fn back_entry(&self) -> Option<(&K, &V)> {
        self.head
            // SAFETY: We guarantee as an invariant a Some(NonNull) is a valid pointer
            .map(|head| unsafe { head.as_ref() })
            .map(|node| (&*node.key, &node.value))
    }

    /// Get the element at the back
    #[inline]
    pub fn back(&self) -> Option<&V> {
        // SAFETY: We guarantee as an invariant a Some(NonNull) is a valid pointer
        self.head.map(|head| &unsafe { head.as_ref() }.value)
    }

    /// Get a mutable reference to the element at the back
    #[inline]
    pub fn back_mut(&mut self) -> Option<&mut V> {
        self.head
            // SAFETY: We guaratnee as an invariant a Some(NonNull) is a valid pointer
            .map(|mut head| &mut unsafe { head.as_mut() }.value)
    }

    /// Remove and return the least recently inserted key and value, if there is one.
    #[inline]
    pub fn pop_front_entry(&mut self) -> Option<(K, V)> {
        self.tail.and_then(|tail| {
            self.base
                // SAFETY: See pop_back_entry() note
                .remove(KeyRefBorrow::from_ref(&*unsafe { tail.as_ref() }.key))
                .map(|mut node| {
                    self.remove_node(&mut node);
                    node
                })
                // SAFETY: remove_node unlinks and removes the node, returning a valid object
                .map(|data| unsafe { Box::from_raw(data.as_ptr()) })
                .map(|data| (*data.key, data.value))
        })
    }

    /// Remove and return the least recently inserted element, if there is one.
    #[inline]
    pub fn pop_front(&mut self) -> Option<V> {
        self.tail.and_then(|tail| {
            self.base
                // SAFETY: See pop_back_entry() note
                .remove(KeyRefBorrow::from_ref(&*unsafe { tail.as_ref() }.key))
                .map(|mut node| {
                    self.remove_node(&mut node);
                    node
                })
                // SAFETY: remove_node unlinks and removes the node, returning a valid object
                .map(|data| unsafe { Box::from_raw(data.as_ptr()) })
                .map(|data| (data.value))
        })
    }

    /// Get the key and value from the element at the front, if there is one
    #[inline]
    pub fn front_entry(&self) -> Option<(&K, &V)> {
        self.tail
            // SAFETY: We guarantee as an invariant a Some(NonNull) is a valid pointer
            .map(|tail| unsafe { tail.as_ref() })
            .map(|node| (&*node.key, &node.value))
    }

    /// Get the element at the front
    #[inline]
    pub fn front(&self) -> Option<&V> {
        // SAFETY: We guarantee as an invariant a Some(NonNull) is a valid pointer
        self.tail.map(|tail| &unsafe { tail.as_ref() }.value)
    }

    /// Get a mutable reference to the element at the front
    #[inline]
    pub fn front_mut(&mut self) -> Option<&mut V> {
        self.tail
            // SAFETY: We guaratnee as an invariant a Some(NonNull) is a valid pointer
            .map(|mut tail| &mut unsafe { tail.as_mut() }.value)
    }

    /// Move an element to the front of the list
    pub fn move_to_front<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let node = self.base.get(KeyRefBorrow::from_ref(key)).copied();
        if let Some(mut node) = node {
            self.remove_node(&mut node);
            self.insert_tail(node);
        }
    }

    /// Move an element to the back of the list
    pub fn move_to_back<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let node = self.base.get(KeyRefBorrow::from_ref(key)).copied();
        if let Some(mut node) = node {
            self.remove_node(&mut node);
            self.insert_head(node);
        }
    }
}

impl<K, V, S> PartialEq for OrderedHashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &OrderedHashMap<K, V, S>) -> bool {
        self.iter()
            .zip(other.iter())
            .all(|((k1, v1), (k2, v2))| k1 == k2 && v1 == v2)
    }
}

impl<K, V, S> Eq for OrderedHashMap<K, V, S>
where
    K: Eq + Hash,
    V: Eq,
    S: BuildHasher,
{
}

// SAFETY: The raw pointer inside the KeyRef is not accessible outside the object. All mutation
// must be done through &mut of the map and so the OrderedHashMap is Send if K/V/S is Send.
unsafe impl<K: Send, V: Send, S: Send> Send for OrderedHashMap<K, V, S> {}

// SAFETY: No interior mutability exists outside of &mut exclusive mutable references so the
// container is Sync if K/V/S is Sync.
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for OrderedHashMap<K, V, S> {}

impl<K, Q, V, S> Index<&Q> for OrderedHashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

#[allow(clippy::undocumented_unsafe_blocks)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertions_gets() {
        let mut map = OrderedHashMap::new();
        map.insert("Foo", 1);
        map.insert("Bar", 2);
        map.insert("Baz", 3);

        assert_eq!(Some(&1), map.get("Foo"));
        assert_eq!(Some(&2), map.get("Bar"));
        assert_eq!(3, map["Baz"]);
        assert_eq!(None, map.get("FooBar"));

        assert_eq!(Some(1), map.remove("Foo"));
        assert_eq!(None, map.get("Foo"));
    }

    #[test]
    fn insertions_dupe() {
        let mut map: OrderedHashMap<_, _> = [("Foo", 1)].into_iter().collect();
        assert_eq!(Some(&1), map.get("Foo"));
        let val = map.insert("Foo", 2);
        assert_eq!(val, Some(1));
        assert_eq!(Some(&2), map.get("Foo"));
    }

    #[test]
    fn clear_is_empty() {
        let mut map: OrderedHashMap<_, _> = [("Foo", 1)].into_iter().collect();
        assert_eq!(Some(&1), map.get("Foo"));
        assert_eq!(1, map.len());
        assert!(!map.is_empty());

        map.clear();
        assert_eq!(None, map.get("Foo"));
        assert_eq!(0, map.len());
        assert!(map.is_empty());

        map.insert("Foo", 2);
        assert_eq!(Some((&"Foo", &2)), map.get_key_value("Foo"));
        assert_eq!(Some((&"Foo", &mut 2)), map.get_key_value_mut("Foo"));
        assert_eq!(1, map.len());
        assert!(!map.is_empty());
    }

    #[test]
    fn get_mutate() {
        let mut map: OrderedHashMap<_, _> = [("Foo", 1)].into_iter().collect();
        let v = map.get_mut("Foo").unwrap();
        *v += 1;
        assert_eq!(map.get("Foo"), Some(&2));
    }

    #[test]
    fn custom_hasher() {
        use std::collections::hash_map::RandomState;
        let mut map = OrderedHashMap::with_hasher(RandomState::new());

        map.insert(String::from("Foo"), 2usize);
        map.insert(String::from("Bar"), 3);
        map.insert(String::from("Baz"), 4);

        assert_eq!(map.get("Foo"), Some(&2));
        assert_eq!(map.get("Bar"), Some(&3));
        assert_eq!(map.get("Baz"), Some(&4));
    }

    #[test]
    fn remove() {
        let mut map: OrderedHashMap<_, _> = [("Foo", 1)].into_iter().collect();

        assert!(map.contains_key("Foo"));

        assert_eq!(map.remove("Foo"), Some(1));

        map.insert("Foo", 1);
        assert_eq!(map.remove_entry("Foo"), Some(("Foo", 1)));
    }

    #[test]
    fn capacity() {
        let map: OrderedHashMap<usize, usize> = OrderedHashMap::new();
        assert_eq!(map.capacity(), 0);

        let mut map: OrderedHashMap<usize, usize> = OrderedHashMap::with_capacity(10);
        assert!(map.capacity() >= 10);

        map.reserve(100);
        assert!(map.capacity() >= 100);

        assert_eq!(
            map.try_reserve(usize::MAX),
            Err(TryReserveError::CapacityOverflow)
        );
    }

    #[test]
    fn pop_ops() {
        let mut map: OrderedHashMap<_, _> =
            [("Foo", 1), ("Bar", 2), ("Baz", 3)].into_iter().collect();

        assert_eq!(map.pop_front(), Some(1));
        assert_eq!(map.pop_back(), Some(3));
        assert_eq!(map.pop_front(), Some(2));

        assert_eq!(map.pop_front(), None);
        assert_eq!(map.pop_back(), None);
    }

    #[test]
    fn pop_entry_ops() {
        let mut map: OrderedHashMap<_, _> =
            [("Foo", 1), ("Bar", 2), ("Baz", 3)].into_iter().collect();

        assert_eq!(map.pop_front_entry(), Some(("Foo", 1)));
        assert_eq!(map.pop_back_entry(), Some(("Baz", 3)));
        assert_eq!(map.pop_front_entry(), Some(("Bar", 2)));

        assert_eq!(map.pop_front_entry(), None);
        assert_eq!(map.pop_back_entry(), None);
    }

    #[test]
    fn front_back() {
        let map: OrderedHashMap<_, _> = [(1, 1), (2, 2)].into_iter().collect();
        assert_eq!(map.front(), Some(&1));
        assert_eq!(map.back(), Some(&2));
    }

    #[test]
    fn equality() {
        let map1: OrderedHashMap<_, _> = [(24usize, 42usize), (16usize, 69usize)]
            .into_iter()
            .collect();

        let map2: OrderedHashMap<_, _> = [(24usize, 42usize), (16usize, 69usize)]
            .into_iter()
            .collect();

        assert_eq!(map1, map2);

        // Same elements, different order
        let map3: OrderedHashMap<_, _> = [(16usize, 69usize), (24usize, 42usize)]
            .into_iter()
            .collect();

        assert_ne!(map1, map3);
        assert_ne!(map2, map3);
    }

    #[test]
    fn iter() {
        let map: OrderedHashMap<_, _> = [("Foo", 1), ("Bar", 2), ("Baz", 3)].into_iter().collect();

        let mut map_iter = map.iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.size_hint(), (3, Some(3)));
        assert_eq!(map_iter.next(), Some((&"Foo", &1)));
        assert_eq!(map_iter.next(), Some((&"Bar", &2)));
        assert_eq!(map_iter.next(), Some((&"Baz", &3)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.size_hint(), (0, Some(0)));
        assert_eq!(map_iter.next(), None);

        let mut map_iter = (&map).into_iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next(), Some((&"Foo", &1)));
        assert_eq!(map_iter.next(), Some((&"Bar", &2)));
        assert_eq!(map_iter.next(), Some((&"Baz", &3)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);

        let mut map_iter = map.into_iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next(), Some(("Foo", 1)));
        assert_eq!(map_iter.next(), Some(("Bar", 2)));
        assert_eq!(map_iter.next(), Some(("Baz", 3)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);
    }

    #[test]
    fn double_ended_iter() {
        let map: OrderedHashMap<_, _> = [("Foo", 1), ("Bar", 2), ("Baz", 3)].into_iter().collect();

        let mut map_iter = map.iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next_back(), Some((&"Baz", &3)));
        assert_eq!(map_iter.next_back(), Some((&"Bar", &2)));
        assert_eq!(map_iter.next_back(), Some((&"Foo", &1)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);

        let mut map_iter = (&map).into_iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next(), Some((&"Foo", &1)));
        assert_eq!(map_iter.next(), Some((&"Bar", &2)));
        assert_eq!(map_iter.next(), Some((&"Baz", &3)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);

        let mut map_iter = map.into_iter();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next_back(), Some(("Baz", 3)));
        assert_eq!(map_iter.next_back(), Some(("Bar", 2)));
        assert_eq!(map_iter.next_back(), Some(("Foo", 1)));
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);
    }

    #[test]
    fn iter_mut() {
        let mut map: OrderedHashMap<_, _> =
            [("Foo", 1), ("Bar", 2), ("Baz", 3)].into_iter().collect();

        for (_, val) in &mut map {
            *val += 1;
        }

        let mut map_iter = map.iter_mut();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next(), Some((&"Foo", &mut 2)));
        assert_eq!(map_iter.next(), Some((&"Bar", &mut 3)));
        assert_eq!(map_iter.next(), Some((&"Baz", &mut 4)));
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);

        let mut map_iter = map.iter_mut();
        assert_eq!(map_iter.len(), 3);
        assert_eq!(map_iter.next_back(), Some((&"Baz", &mut 4)));
        assert_eq!(map_iter.next_back(), Some((&"Bar", &mut 3)));
        assert_eq!(map_iter.next_back(), Some((&"Foo", &mut 2)));
        assert_eq!(map_iter.next_back(), None);
        assert_eq!(map_iter.len(), 0);
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);
    }

    #[test]
    fn keys() {
        let map: OrderedHashMap<_, _> = [(1, 2), (2, 2)].into_iter().collect();

        let mut map_iter = map.keys();
        assert_eq!(map_iter.size_hint(), (2, Some(2)));
        assert_eq!(map_iter.next(), Some(&1));
        assert_eq!(map_iter.next_back(), Some(&2));
        assert_eq!(map_iter.size_hint(), (0, Some(0)));
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);

        let mut map_iter = map.into_keys();
        assert_eq!(map_iter.next(), Some(1));
        assert_eq!(map_iter.next_back(), Some(2));
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);
    }

    #[test]
    fn values() {
        let map: OrderedHashMap<_, _> = [(1, 2), (2, 2)].into_iter().collect();

        let mut map_iter = map.values();
        assert_eq!(map_iter.size_hint(), (2, Some(2)));
        assert_eq!(map_iter.next(), Some(&2));
        assert_eq!(map_iter.next_back(), Some(&2));
        assert_eq!(map_iter.size_hint(), (0, Some(0)));
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);

        let mut map = map;
        for v in map.values_mut() {
            *v += 1;
        }

        let map = map;

        let mut map_iter = map.into_values();
        assert_eq!(map_iter.next(), Some(3));
        assert_eq!(map_iter.next_back(), Some(3));
        assert_eq!(map_iter.next(), None);
        assert_eq!(map_iter.next_back(), None);
    }

    #[test]
    fn borrow_from_iter() {
        let map: OrderedHashMap<usize, usize> = [(&1, &2), (&2, &2)].into_iter().collect();

        let mut map_iter = map.iter();
        assert_eq!(map_iter.next(), Some((&1, &2)));
    }

    #[test]
    fn drain() {
        let mut map: OrderedHashMap<usize, usize> = [(1, 2), (3, 4)].into_iter().collect();

        let mut drain_iter = map.drain();
        assert_eq!(drain_iter.next(), Some((1, 2)));
        drop(drain_iter);

        assert_eq!(map.len(), 0);
    }

    #[test]
    fn debug() {
        let map: OrderedHashMap<_, _> = [(1, 2), (2, 2)].into_iter().collect();
        assert_eq!(format!("{:?}", map), "{1: 2, 2: 2}");
    }
}
