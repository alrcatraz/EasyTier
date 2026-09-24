extern crate alloc;

use alloc::boxed::Box;
use core::fmt;
use core::iter::FusedIterator;
use core::marker::PhantomData;

use crate::map::detail::NullableNode;

/// An ordered iterator over the elements of the OrderedHashMap.
///
/// Created by the [`iter`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`iter`]: super::OrderedHashMap::iter
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct Iter<'a, K, V> {
    head: NullableNode<K, V>,
    tail: NullableNode<K, V>,
    len: usize,
    marker: PhantomData<(&'a K, &'a V)>,
}

impl<K, V> Clone for Iter<'_, K, V> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

impl<K, V> fmt::Debug for Iter<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((*self).clone()).finish()
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.tail {
            None => None,
            Some(tail) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { tail.as_ref() };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    self.tail = node.prev;
                }
                Some((&node.key, &node.value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> DoubleEndedIterator for Iter<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.head {
            None => None,
            Some(head) => {
                self.len -= 1; // TODO: Use len check? Should be zero when finished iterating

                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { head.as_ref() };
                if self.tail == self.head {
                    self.tail = None;
                    self.head = None;
                } else {
                    self.head = node.next;
                }
                Some((&node.key, &node.value))
            }
        }
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K, V, S> IntoIterator for &'a super::OrderedHashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            head: self.head,
            tail: self.tail,
            len: self.base.len(),
            marker: PhantomData,
        }
    }
}

// SAFETY: Iter borrows from the OrderedHashMap like a reference. Since OrderedHashMap has no
// interior mutability outside &mut references it cannot be mutated while this Iter lives. So as
// long as K/V/S is Send, the container is Send.
unsafe impl<K: Send, V: Send> Send for Iter<'_, K, V> {}

// SAFETY: &T is Send so Sync is safe as well as long as K/V/S is Sync.
unsafe impl<K: Sync, V: Sync> Sync for Iter<'_, K, V> {}

/// An ordered iterator over the elements of the OrderedHashMap.
///
/// Created by the [`iter_mut`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`iter_mut`]: super::OrderedHashMap::iter_mut
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct IterMut<'a, K, V> {
    head: NullableNode<K, V>,
    tail: NullableNode<K, V>,
    len: usize,
    marker: PhantomData<(&'a K, &'a mut V)>,
}

impl<'a, K, V> IterMut<'a, K, V> {
    fn iter(&self) -> Iter<'a, K, V> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            marker: PhantomData,
        }
    }
}

impl<K, V> fmt::Debug for IterMut<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.tail {
            None => None,
            Some(mut tail) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { tail.as_mut() };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    self.tail = node.prev;
                }
                Some((&node.key, &mut node.value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> DoubleEndedIterator for IterMut<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.head {
            None => None,
            Some(mut head) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { head.as_mut() };
                if self.tail == self.head {
                    self.tail = None;
                    self.head = None;
                } else {
                    self.head = node.next;
                }
                Some((&node.key, &mut node.value))
            }
        }
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {}

impl<K, V> FusedIterator for IterMut<'_, K, V> {}

impl<'a, K, V, S> IntoIterator for &'a mut super::OrderedHashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            head: self.head,
            tail: self.tail,
            len: self.base.len(),
            marker: PhantomData,
        }
    }
}

// SAFETY: IterMut is created by calling a &mut function (i.e. exclusive mutable reference) and its
// lifetime is tied to the original Map. This guarantees this object has exclusive mutability on
// the object and makes it Send safe as longas K/V are Send.
unsafe impl<K: Send, V: Send> Send for IterMut<'_, K, V> {}

// SAFETY: &T is Send so Sync is safe as well as long as K/V is Sync.
unsafe impl<K: Sync, V: Sync> Sync for IterMut<'_, K, V> {}

/// An ordered iterator over the elements of the OrderedHashMap.
///
/// Created by the [`into_iter`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`into_iter`]: super::OrderedHashMap::into_iter
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct IntoIter<K, V> {
    head: NullableNode<K, V>,
    tail: NullableNode<K, V>,
    len: usize,
    marker: PhantomData<(K, V)>,
}

impl<K, V> IntoIter<K, V> {
    pub(crate) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            marker: PhantomData,
        }
    }
}

impl<K, V> fmt::Debug for IntoIter<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K, V> Drop for IntoIter<K, V> {
    fn drop(&mut self) {
        while self.len > 0 {
            self.next();
        }
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.tail {
            None => None,
            Some(mut tail) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { Box::from_raw(tail.as_mut()) };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    // SAFETY: Already checked head != tail so prev exists
                    unsafe {
                        node.prev.unwrap().as_mut().next = None;
                    }
                    self.tail = node.prev;
                }
                Some((*node.key, node.value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.head {
            None => None,
            Some(mut head) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { Box::from_raw(head.as_mut()) };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    // SAFETY: Already checked head != tail so next exists
                    unsafe {
                        node.next.unwrap().as_mut().prev = None;
                    }
                    self.head = node.next;
                }
                Some((*node.key, node.value))
            }
        }
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}

impl<K, V> FusedIterator for IntoIter<K, V> {}

impl<K, V, S> IntoIterator for super::OrderedHashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(mut self) -> Self::IntoIter {
        let (head, tail, len) = (self.head, self.tail, self.len());
        // SAFETY: base always exists while self exists except here, but here we forget(self)
        // right after.
        unsafe {
            // Drop base since we won't be calling Drop. Can't call drop() directly as it is not
            // Copy and drop takes a self, not a &mut self.
            core::ptr::drop_in_place(&mut self.base);
        }
        // Forget instead of Drop. If we called Drop it would free all the nodes but we explicitly
        // want to keep the nodes alive. We dropped the HashMap already.
        core::mem::forget(self);
        Self::IntoIter {
            head,
            tail,
            len,
            marker: PhantomData,
        }
    }
}

// SAFETY: IntoIter takes ownership of the data and offers no mutability whatsoever. This makes the
// type Send.
unsafe impl<K: Send, V: Send> Send for IntoIter<K, V> {}

// SAFETY: &T is Send so Sync is safe as well.
unsafe impl<K: Sync, V: Sync> Sync for IntoIter<K, V> {}

/// An ordered, draining iterator over the elements of the OrderedHashMap.
///
/// Clears the container and leaves the capacity unchanged for re-use. If this iterator is dropped
/// before fully consuming all the elements they are freed.
/// Created by the [`drain`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`drain`]: super::OrderedHashMap::drain
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct Drain<'a, K, V> {
    pub(super) head: NullableNode<K, V>,
    pub(super) tail: NullableNode<K, V>,
    pub(super) len: usize,
    pub(super) marker: PhantomData<&'a mut (K, V)>,
}

impl<K, V> Drain<'_, K, V> {
    pub(crate) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            marker: PhantomData,
        }
    }
}

impl<K, V> fmt::Debug for Drain<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K, V> Drop for Drain<'_, K, V> {
    fn drop(&mut self) {
        while self.len > 0 {
            self.next();
        }
    }
}

impl<K, V> Iterator for Drain<'_, K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.tail {
            None => None,
            Some(mut tail) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { Box::from_raw(tail.as_mut()) };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    // SAFETY: Already checked head != tail so prev exists
                    unsafe {
                        node.prev.unwrap().as_mut().next = None;
                    }
                    self.tail = node.prev;
                }
                Some((*node.key, node.value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> DoubleEndedIterator for Drain<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.head {
            None => None,
            Some(mut head) => {
                self.len -= 1;
                // SAFETY: Accessing a NonNull is safe it was created correctly.
                let node = unsafe { Box::from_raw(head.as_mut()) };
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    // SAFETY: Already checked head != tail so next exists
                    unsafe {
                        node.next.unwrap().as_mut().prev = None;
                    }
                    self.head = node.next;
                }
                Some((*node.key, node.value))
            }
        }
    }
}

impl<K, V> ExactSizeIterator for Drain<'_, K, V> {}

impl<K, V> FusedIterator for Drain<'_, K, V> {}

// SAFETY: Drain takes ownership of the data and offers no mutability whatsoever. This makes the
// type Send.
unsafe impl<K: Send, V: Send> Send for Drain<'_, K, V> {}

// SAFETY: &T is Send so Sync is safe as well.
unsafe impl<K: Sync, V: Sync> Sync for Drain<'_, K, V> {}

/// An ordered iterator over the keys of the OrderedHashMap.
///
/// Created by the [`keys`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`keys`]: super::OrderedHashMap::keys
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct Keys<'a, K, V> {
    pub(crate) inner: Iter<'a, K, V>,
}

impl<K, V> Clone for Keys<'_, K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> fmt::Debug for Keys<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((*self).clone()).finish()
    }
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for Keys<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {}

impl<K, V> FusedIterator for Keys<'_, K, V> {}

/// An ordered iterator over the keys of the OrderedHashMap.
///
/// Created by the [`into_keys`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`into_keys`]: super::OrderedHashMap::into_keys
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct IntoKeys<K, V> {
    pub(crate) inner: IntoIter<K, V>,
}

impl<K, V> fmt::Debug for IntoKeys<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(k, _)| k))
            .finish()
    }
}

impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoKeys<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V> ExactSizeIterator for IntoKeys<K, V> {}

impl<K, V> FusedIterator for IntoKeys<K, V> {}

/// An ordered iterator over the values of the OrderedHashMap.
///
/// Created by the [`values`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`values`]: super::OrderedHashMap::values
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct Values<'a, K, V> {
    pub(crate) inner: Iter<'a, K, V>,
}

impl<K, V> Clone for Values<'_, K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V> fmt::Debug for Values<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((*self).clone()).finish()
    }
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for Values<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {}

impl<K, V> FusedIterator for Values<'_, K, V> {}

/// An ordered iterator over the values of the OrderedHashMap.
///
/// Created by the [`into_values`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`into_values`]: super::OrderedHashMap::into_values
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct IntoValues<K, V> {
    pub(crate) inner: IntoIter<K, V>,
}

impl<K, V> fmt::Debug for IntoValues<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(_, v)| v))
            .finish()
    }
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IntoValues<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for IntoValues<K, V> {}

impl<K, V> FusedIterator for IntoValues<K, V> {}

/// An ordered mutable iterator over the values of the OrderedHashMap.
///
/// Created by the [`into_values`] method of [`OrderedHashMap`]. See its documentation for details.
///
/// [`into_values`]: super::OrderedHashMap::into_values
/// [`OrderedHashMap`]: super::OrderedHashMap
pub struct ValuesMut<'a, K, V> {
    pub(crate) inner: IterMut<'a, K, V>,
}

impl<K, V> fmt::Debug for ValuesMut<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.inner.iter().map(|(_, v)| v))
            .finish()
    }
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

impl<K, V> DoubleEndedIterator for ValuesMut<'_, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {}

impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}

impl<'a, K, V, S> Extend<&'a (K, V)> for super::OrderedHashMap<K, V, S>
where
    K: Eq + core::hash::Hash + Copy,
    V: Copy,
    S: core::hash::BuildHasher,
{
    fn extend<T: IntoIterator<Item = &'a (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(*k, *v);
        }
    }
}

impl<'a, K, V, S> Extend<(&'a K, &'a V)> for super::OrderedHashMap<K, V, S>
where
    K: Eq + core::hash::Hash + Copy,
    V: Copy,
    S: core::hash::BuildHasher,
{
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(*k, *v);
        }
    }
}

impl<K, V, S> Extend<(K, V)> for super::OrderedHashMap<K, V, S>
where
    K: Eq + core::hash::Hash,
    S: core::hash::BuildHasher,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K, V, S> FromIterator<(K, V)> for super::OrderedHashMap<K, V, S>
where
    K: Eq + core::hash::Hash,
    S: core::hash::BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut lhm = Self::default();
        lhm.extend(iter);
        lhm
    }
}

impl<'a, K, V, S> FromIterator<(&'a K, &'a V)> for super::OrderedHashMap<K, V, S>
where
    K: Eq + core::hash::Hash + Copy,
    V: Copy,
    S: core::hash::BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (&'a K, &'a V)>>(iter: I) -> Self {
        let mut lhm = Self::default();
        lhm.extend(iter);
        lhm
    }
}

#[cfg(test)]
mod tests {
    use super::super::OrderedHashMap;

    #[test]
    fn debug() {
        let map: OrderedHashMap<_, _> = [(1, 3), (2, 4)].into_iter().collect();

        assert_eq!(format!("{:?}", map.iter()), "[(1, 3), (2, 4)]");
        assert_eq!(format!("{:?}", map.keys()), "[1, 2]");
        let mut map_clone = map.clone();
        assert_eq!(format!("{:?}", map_clone.drain()), "[(1, 3), (2, 4)]");
        let map_clone = map.clone();
        assert_eq!(format!("{:?}", map_clone.into_keys()), "[1, 2]");
        assert_eq!(format!("{:?}", map.values()), "[3, 4]");
        let map_clone = map.clone();
        assert_eq!(format!("{:?}", map_clone.into_values()), "[3, 4]");
        let mut map = map;
        assert_eq!(format!("{:?}", map.iter_mut()), "[(1, 3), (2, 4)]");
        assert_eq!(format!("{:?}", map.values_mut()), "[3, 4]");
        assert_eq!(format!("{:?}", map.into_iter()), "[(1, 3), (2, 4)]");
    }

    #[test]
    fn size_hints() {
        let map: OrderedHashMap<_, _> = [(1, 3), (2, 4)].into_iter().collect();

        assert_eq!(map.iter().size_hint(), (2, Some(2)));
        assert_eq!(map.clone().into_iter().size_hint(), (2, Some(2)));
        assert_eq!(map.clone().iter_mut().size_hint(), (2, Some(2)));
        assert_eq!(map.clone().drain().size_hint(), (2, Some(2)));
    }
}
