use super::OrderedHashSet;

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;

impl<T, S> Serialize for OrderedHashSet<T, S>
where
    T: Eq + Hash + Serialize,
    S: BuildHasher,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for t in self {
            seq.serialize_element(t)?;
        }
        seq.end()
    }
}

struct OrderedHashSetVisitor<T, S> {
    marker: PhantomData<OrderedHashSet<T, S>>,
}

impl<'de, T> Deserialize<'de> for OrderedHashSet<T>
where
    T: Eq + Hash + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(OrderedHashSetVisitor {
            marker: PhantomData,
        })
    }
}

impl<'de, T, S> Visitor<'de> for OrderedHashSetVisitor<T, S>
where
    T: Eq + Hash + Deserialize<'de>,
    S: BuildHasher + Default,
{
    type Value = OrderedHashSet<T, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values =
            Self::Value::with_capacity_and_hasher(seq.size_hint().unwrap_or(0), S::default());
        while let Some(value) = seq.next_element()? {
            values.insert(value);
        }
        Ok(values)
    }
}
