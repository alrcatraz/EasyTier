extern crate alloc;

use alloc::boxed::Box;
use core::borrow::Borrow;
use core::hash::{Hash, Hasher};
use core::ptr::NonNull;

pub(super) type NonNullNode<K, V> = NonNull<Node<K, V>>;
pub(super) type NullableNode<K, V> = Option<NonNullNode<K, V>>;

#[derive(Debug)]
pub(super) struct Node<K, V> {
    pub(super) key: Box<K>,
    pub(super) value: V,
    pub(super) next: NullableNode<K, V>,
    pub(super) prev: NullableNode<K, V>,
}

impl<K, V> Node<K, V> {
    pub(super) fn new(key: K, value: V) -> Self {
        Self {
            key: Box::new(key),
            value,
            next: None,
            prev: None,
        }
    }
}

#[derive(Debug)]
pub(super) struct KeyRef<K> {
    pub(super) key: *const K,
}

impl<K: Hash> Hash for KeyRef<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: KeyRef holds a NonNull  which MUST be a valid pointer if it was created
        // correctly. i.e., if this fails it is indicative of a problem in insert()
        unsafe { &*self.key }.hash(state);
    }
}

impl<K: PartialEq> PartialEq for KeyRef<K> {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: KeyRef holds a NonNull  which MUST be a valid pointer if it was created
        // correctly. i.e., if this fails it is indicative of a problem in insert()
        unsafe { &*self.key }.eq(unsafe { &*other.key })
    }
}

impl<K: Eq> Eq for KeyRef<K> {}

// By introducing my own new type I can enforce my unique Borrow because I own
// both the borrow from and to.
//
// By making the repr transparent the compiler guarantees KeyRefBorrow<Q> has the
// exact same memory layout as Q. This allows me to derive Hash, PartialEq, and Eq
// safely. More importantly, it allows me to transmute between Q and KeyRefBorrow<Q>.
// Not sure if there is any way to solve this problem without a transmute despite
// my best efforts.
#[derive(Debug, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub(super) struct KeyRefBorrow<Q: ?Sized>(Q);

impl<Q, K> Borrow<KeyRefBorrow<Q>> for KeyRef<K>
where
    K: Borrow<Q>,
    Q: ?Sized,
{
    fn borrow(&self) -> &KeyRefBorrow<Q> {
        // SAFETY: KeyRef holds a NonNull  which MUST be a valid pointer if it was created
        // correctly. i.e., if this fails it is indicative of a problem in insert()
        KeyRefBorrow::from_ref(unsafe { &*self.key }.borrow())
    }
}

impl<Q: ?Sized> KeyRefBorrow<Q> {
    pub(super) const fn from_ref(q: &Q) -> &Self {
        // SAFETY: KeyRefBorrow is repr transparent which guarantees it has the same layout as its
        // single member making the transmute safe and a no-op.
        unsafe { core::mem::transmute(q) }
    }
}

#[allow(clippy::undocumented_unsafe_blocks)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyref_traits() {
        use std::boxed::Box;
        use std::collections::hash_map::RandomState;
        use std::hash::BuildHasher;

        let s = RandomState::new();

        // Different pointers, same data
        // Should be equal and hash the same
        let box1 = Box::new("KeyName");
        let box2 = Box::new("KeyName");
        let keyref1 = KeyRef { key: &*box1 };
        let keyref2 = KeyRef { key: &*box2 };

        assert_eq!(keyref1, keyref2);
        // TODO: use BuildHasher::build_one() when stabilized
        let mut hasher = s.build_hasher();
        keyref1.hash(&mut hasher);
        let hash1 = hasher.finish();

        let mut hasher = s.build_hasher();
        keyref2.hash(&mut hasher);
        let hash2 = hasher.finish();
        assert_eq!(hash1, hash2);

        //Different data
        // Should NOT be equal
        let box3 = Box::new("OtherName");
        let keyref2 = KeyRef { key: &*box3 };

        assert_ne!(keyref1, keyref2);
        let mut hasher = s.build_hasher();
        keyref2.hash(&mut hasher);
        let hash2 = hasher.finish();
        assert_ne!(hash1, hash2);

        // Drop boxes here to make sure KeyRef pointers are valid for duration of tests
        drop(box1);
        drop(box2);
        drop(box3);
    }

    #[test]
    fn node_new() {
        let node = Node::new("blah".to_string(), 42usize);

        assert_eq!(*node.key, "blah");
        assert_eq!(node.value, 42usize);
        assert_eq!(node.next, None);
        assert_eq!(node.prev, None);
    }
}
