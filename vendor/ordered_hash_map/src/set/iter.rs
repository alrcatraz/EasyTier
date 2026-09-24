use core::fmt;
use core::iter::FusedIterator;

/// An ordered iterator over the elements.
///
/// Created by the [`iter`] method of [`OrderedHashSet`]. See its documentation for details.
///
/// [`iter`]: super::OrderedHashSet::iter
/// [`OrderedHashSet`]: super::OrderedHashSet
pub struct Iter<'a, T> {
    inner: crate::ordered_map::Iter<'a, T, ()>,
}

impl<'a, T> Iter<'a, T> {
    fn return_key(val: (&'a T, &())) -> &'a T {
        val.0
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> fmt::Debug for Iter<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((*self).clone()).finish()
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Self::return_key)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(Self::return_key)
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<T> FusedIterator for Iter<'_, T> {}

impl<'a, T, S> IntoIterator for &'a super::OrderedHashSet<T, S> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self.map.iter(),
        }
    }
}

/// An ordered iterator over the elements.
///
/// Created by the [`into_iter`] method of [`OrderedHashSet`]. See its documentation for details.
///
/// [`into_iter`]: super::OrderedHashSet::into_iter
/// [`OrderedHashSet`]: super::OrderedHashSet
pub struct IntoIter<T> {
    inner: crate::ordered_map::IntoIter<T, ()>,
}

impl<T> IntoIter<T> {
    fn return_key(val: (T, ())) -> T {
        val.0
    }
}

impl<T> IntoIter<T> {
    fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.inner.iter(),
        }
    }
}

impl<T> fmt::Debug for IntoIter<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Self::return_key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(Self::return_key)
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> FusedIterator for IntoIter<T> {}

impl<T, S> IntoIterator for super::OrderedHashSet<T, S> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self.map.into_iter(),
        }
    }
}

/// An ordered, draining iterator over the elements.
///
/// Clears the container and leaves the capacity unchanged for re-use. If this iterator is dropped
/// before fully consuming all the elements they are freed.
/// Created by the [`drain`] method of [`OrderedHashSet`]. See its documentation for details.
///
/// [`drain`]: super::OrderedHashSet::drain
/// [`OrderedHashSet`]: super::OrderedHashSet
pub struct Drain<'a, T> {
    pub(crate) inner: crate::ordered_map::Drain<'a, T, ()>,
}

impl<T> Drain<'_, T> {
    fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.inner.iter(),
        }
    }
}

impl<T> fmt::Debug for Drain<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(x, _)| x)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(x, _)| x)
    }
}

impl<T> ExactSizeIterator for Drain<'_, T> {}

impl<T> FusedIterator for Drain<'_, T> {}

impl<'a, T, S> Extend<&'a T> for super::OrderedHashSet<T, S>
where
    T: 'a + Eq + core::hash::Hash + Copy,
    S: core::hash::BuildHasher,
{
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.map.extend(iter.into_iter().copied().map(|x| (x, ())));
    }
}

impl<T, S> Extend<T> for super::OrderedHashSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: core::hash::BuildHasher,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.map.extend(iter.into_iter().map(|x| (x, ())));
    }
}

impl<T, S> FromIterator<T> for super::OrderedHashSet<T, S>
where
    T: Eq + core::hash::Hash,
    S: core::hash::BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut lhs = Self::default();
        lhs.extend(iter);
        lhs
    }
}

impl<'a, T, S> FromIterator<&'a T> for super::OrderedHashSet<T, S>
where
    T: 'a + Eq + core::hash::Hash + Copy,
    S: core::hash::BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        let mut lhs = Self::default();
        lhs.extend(iter);
        lhs
    }
}

#[cfg(test)]
mod tests {
    use super::super::OrderedHashSet;

    #[test]
    fn extend_and_drain() {
        let mut set1: OrderedHashSet<i32> = [1, 2, 3].into_iter().collect();
        let set2: OrderedHashSet<i32> = [4, 5, 6].into_iter().collect();

        set1.extend(set2.into_iter());

        assert_eq!(set1, [1, 2, 3, 4, 5, 6].into_iter().collect());

        let mut drain = set1.drain();
        assert_eq!(drain.size_hint(), (6, Some(6)));
        assert_eq!(format!("{:?}", drain), "[1, 2, 3, 4, 5, 6]");
        assert_eq!(drain.next(), Some(1));
        drop(drain); //Drop without consuming all elements
        assert_eq!(set1.len(), 0);
    }

    #[test]
    fn iters() {
        let set1: OrderedHashSet<i32> = [1, 2, 3].into_iter().collect();

        let mut iter = set1.iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));
        assert_eq!(format!("{:?}", iter), "[1, 2, 3]");
        assert_eq!(iter.next(), Some(&1));

        let mut into_iter = set1.into_iter();
        assert_eq!(into_iter.size_hint(), (3, Some(3)));
        assert_eq!(format!("{:?}", into_iter), "[1, 2, 3]");
        assert_eq!(into_iter.next(), Some(1));
    }
}
