use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::ops::Index;

pub use hashbrown::DefaultHashBuilder;
pub use hashbrown::TryReserveError;

use crate::ordered_map;

mod iter;
pub use iter::{Drain, IntoIter, Iter};

#[cfg(feature = "serde")]
mod serde;

/// A hash set which preserves the order of insertion.
///
/// Backed by [`hashbrown`] and its default hashing algorithm, currently [`AHash`]. The hashing
/// algorithm can be changed on a per-map basis with [`with_hasher`] and [`with_capacity_and_hasher`].
/// More details on hashing algorithms and why AHash was selected in the [`hashbrown`] crate.
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
/// This container has some additional memory overhead on top of its backing HashMap. [`OrderedHashSet`]
/// holds two pointers to maintain the linked list. Each Value inserted has the overhead of three pointers, one for the key
/// and two for the linked list properties.
///
/// [`AHash`]: https://crates.io/crates/ahash
/// [`hashbrown`]: https://crates.io/crates/hashbrown
/// [`with_hasher`]: Self::with_hasher
/// [`with_capacity_and_hasher`]: Self::with_capacity_and_hasher
/// [`HashMap`]: std::collections::HashMap
/// [`RandomState`]: std::collections::hash_map::RandomState
pub struct OrderedHashSet<T, S = DefaultHashBuilder> {
    map: ordered_map::OrderedHashMap<T, (), S>,
}

impl<T, S> fmt::Debug for OrderedHashSet<T, S>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T, S> Clone for OrderedHashSet<T, S>
where
    T: Hash + Eq + Clone,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl<T, S> Default for OrderedHashSet<T, S>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<T> OrderedHashSet<T, DefaultHashBuilder> {
    /// Create an empty set.
    ///
    /// Capacity is 0 so the set will not allocated until inserted into.
    /// ```
    /// use ordered_hash_map::OrderedHashSet;
    /// let set = OrderedHashSet::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty set with at least the specified capacity.
    ///
    /// The map will allocate if capacity is nonzero.
    /// ```
    /// use ordered_hash_map::OrderedHashSet;
    /// let set = OrderedHashSet::<i32>::with_capacity(10);
    /// assert!(set.capacity() >= 10)
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: ordered_map::OrderedHashMap::with_capacity(capacity),
        }
    }

    /// Returns a draining iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    /// ```
    /// # use ordered_hash_map::OrderedHashSet;
    /// let mut set: OrderedHashSet<i32> = [1i32, 2, 3, 4, 5].iter().collect();
    /// assert_eq!(set.drain().next(), Some(1));
    /// assert!(set.is_empty());
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        Drain {
            inner: self.map.drain(),
        }
    }
}

impl<T, S> OrderedHashSet<T, S> {
    /// Returns a draining iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    /// ```
    /// # use ordered_hash_map::OrderedHashSet;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rs = RandomState::new();
    /// let set = OrderedHashSet::<i32, _>::with_hasher(rs);
    /// ```
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: ordered_map::OrderedHashMap::with_hasher(hash_builder),
        }
    }

    /// Create an empty set with the given capacity and which will use the given hasher.
    ///
    /// The map will allocate if capacity is nonzero.
    /// ```no_run
    /// # use ordered_hash_map::OrderedHashSet;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rs = RandomState::new();
    /// let set = OrderedHashSet::<i32, _>::with_capacity_and_hasher(10, rs);
    /// assert!(set.capacity() >= 10)
    /// ```
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: ordered_map::OrderedHashMap::with_capacity_and_hasher(capacity, hash_builder),
        }
    }

    /// Returns a reference to the sets hasher.
    /// ```
    /// # use ordered_hash_map::OrderedHashSet;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rs = RandomState::new();
    /// let set = OrderedHashSet::<i32, _>::with_hasher(rs);
    /// let rs: &RandomState = set.hasher();
    /// ```
    pub fn hasher(&self) -> &S {
        self.map.hasher()
    }

    /// The number of elements the set can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// The number of elements currently held in the set.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Removes all values from the set.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns an iterator over the elements in insertion order.
    ///
    /// # Performance
    /// This iterator visits each element once instead of each bucket once. This iterator is O(len)
    /// which a usual map iterator is O(capacity) and may visit empty buckets.
    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }
}

impl<T, S> OrderedHashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    /// Insert a new element into the set. Returns whether the key already existed.
    /// ```
    /// # use ordered_hash_map::OrderedHashSet;
    /// let mut set = OrderedHashSet::<i32>::new();
    /// assert!(!set.insert(4));
    /// assert!(set.insert(4));
    /// ```
    pub fn insert(&mut self, value: T) -> bool {
        self.map.insert(value, ()).is_some()
    }

    /// Returns a reference to the value stored in the set.
    /// ```
    /// # use ordered_hash_map::OrderedHashSet;
    /// let mut set = OrderedHashSet::<String>::new();
    /// set.insert("A".to_string());
    /// // Stored as String but can use &str to get
    /// assert_eq!(set.get("A"), Some(&String::from("A")));
    /// ```
    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get_key_value(value).map(|(x, _)| x)
    }

    /// Remove an element from the set, returning whether or not the element existed.
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove(value).is_some()
    }

    /// Remove and return an element from the set
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove_entry(value).map(|(x, _)| x)
    }

    /// Check if the set contains this key.
    pub fn contains<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(value)
    }

    /// Reserves capacity for at least `additional` more elements.
    ///
    /// # Panics
    /// Panics if the new size would overflow a usize.
    pub fn reserve(&mut self, additional: usize) {
        self.map.reserve(additional);
    }

    /// Tries to reserve capacity for at least `additional` more elements.
    ///
    /// # Errors
    /// Returns an error if the new size would overflow a usize.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.map.try_reserve(additional)
    }

    /// Shrinks the map as much as possible. May leave some space in accordance with the resize
    /// policy.
    pub fn shrink_to_fit(&mut self) {
        self.map.shrink_to_fit();
    }

    /// Shrinks the map down to a lower limit. May leave some space in accordance with the resize
    /// policy.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.map.shrink_to(min_capacity);
    }

    /// Removes and returns the most recently inserted value, if there is one.
    pub fn pop_back(&mut self) -> Option<T> {
        self.map.pop_back_entry().map(|(x, _)| x)
    }

    /// Returns the most recently inserted value, if there is one
    pub fn back(&self) -> Option<&T> {
        self.map.back_entry().map(|(x, _)| x)
    }

    /// Remove and return the least recently inserted value, if there is one.
    pub fn pop_front(&mut self) -> Option<T> {
        self.map.pop_front_entry().map(|(x, _)| x)
    }

    /// Returns the most recently inserted value, if there is one
    pub fn front(&self) -> Option<&T> {
        self.map.front_entry().map(|(x, _)| x)
    }

    /// Move an element to the front of the list
    pub fn move_to_front<Q>(&mut self, key: &Q)
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.move_to_front(key);
    }

    /// Move and element to the back of the list
    pub fn move_to_back<Q>(&mut self, key: &Q)
    where
        T: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.move_to_back(key);
    }
}

impl<T, S> PartialEq for OrderedHashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
    fn eq(&self, other: &OrderedHashSet<T, S>) -> bool {
        self.map.eq(&other.map)
    }
}

impl<T, S> Eq for OrderedHashSet<T, S>
where
    T: Eq + Hash,
    S: BuildHasher,
{
}

impl<T, Q, S> Index<&Q> for OrderedHashSet<T, S>
where
    T: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    S: BuildHasher,
{
    type Output = T;

    fn index(&self, key: &Q) -> &T {
        self.get(key).expect("no entry found for key")
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedHashSet;
    #[test]
    fn default() {
        let set = OrderedHashSet::<i32>::new();
        assert_eq!(set.len(), 0);
        assert_eq!(set.iter().size_hint(), (0, Some(0)));
        assert_eq!(set.iter().next(), None);
        assert_eq!(set.iter().next_back(), None);
    }

    #[test]
    fn clone() {
        let set: OrderedHashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        let set_clone = set.clone();

        assert_eq!(set, set_clone);
        assert_eq!(format!("{:?}", set), format!("{:?}", set_clone));

        set.iter()
            .zip(set_clone.iter())
            .for_each(|(val, val_clone)| assert_eq!(val, val_clone));

        assert_eq!(set.len(), 4);

        set.iter()
            .rev()
            .zip(set_clone.iter().rev())
            .for_each(|(val, val_clone)| assert_eq!(val, val_clone));
    }

    #[test]
    fn sizes() {
        let mut set: OrderedHashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        assert_eq!(set.len(), 4);
        assert!(!set.is_empty());

        let mut drain = set.drain();
        assert_eq!(drain.next(), Some(1));
        assert_eq!(drain.next_back(), Some(4));

        drop(drain); // Drop without consuming all of it

        assert_eq!(set.len(), 0);
        assert!(set.is_empty());

        let mut set: OrderedHashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        set.clear();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn insert_remove_ops() {
        let mut set = OrderedHashSet::<i32>::with_capacity(10);
        assert!(set.capacity() >= 10);
        set.reserve(15);
        assert!(set.capacity() >= 25);

        // First try on empty sets
        assert!(!set.remove(&1));
        assert!(set.get(&2).is_none());
        assert!(set.take(&3).is_none());
        assert!(set.back().is_none());
        assert!(set.pop_back().is_none());
        assert!(set.front().is_none());
        assert!(set.pop_front().is_none());

        set.move_to_front(&1); // Don't crash on non existent element

        let mut set: OrderedHashSet<i32> = [1, 2, 3, 4].into_iter().collect();
        assert!(!set.contains(&5));
        assert!(!set.insert(5)); // [1, 2, 3, 4, 5]
        assert!(set.insert(5)); // no change
        assert!(set.contains(&5));
        assert!(set.remove(&1)); // [2, 3, 4, 5]
        assert_eq!(set[&2], 2); // no change
        assert_eq!(set.take(&3), Some(3)); // [2, 4, 5]
        assert_eq!(set.back(), Some(&5)); // no change
        assert_eq!(set.pop_back(), Some(5)); // [2, 4]
        assert_eq!(set.front(), Some(&2)); // no change
        assert_eq!(set.pop_front(), Some(2)); // [4]
        assert_eq!(set.get(&4), set.back());
        assert_eq!(set.front(), set.back());
    }

    #[test]
    fn move_to() {
        let mut set: OrderedHashSet<i32> = [1, 2, 3].into_iter().collect();
        set.move_to_front(&2);
        assert_eq!(set, [2, 1, 3].into_iter().collect());
        set.move_to_back(&2);
        assert_eq!(set, [1, 3, 2].into_iter().collect());
    }
}
