use super::OrderedHashMap;
use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeMap, Serializer};

use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;

impl<K, V, S> Serialize for OrderedHashMap<K, V, S>
where
    K: Eq + Hash + Serialize,
    V: Serialize,
    S: BuildHasher,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

struct OrderedHashMapVisitor<K, V, S> {
    marker: PhantomData<OrderedHashMap<K, V, S>>,
}

impl<'de, K, V> Deserialize<'de> for OrderedHashMap<K, V>
where
    K: Eq + Hash + Deserialize<'de>,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(OrderedHashMapVisitor {
            marker: PhantomData,
        })
    }
}

impl<'de, K, V, S> Visitor<'de> for OrderedHashMapVisitor<K, V, S>
where
    K: Eq + Hash + Deserialize<'de>,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    type Value = OrderedHashMap<K, V, S>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values =
            Self::Value::with_capacity_and_hasher(map.size_hint().unwrap_or(0), S::default());
        while let Some((key, value)) = map.next_entry()? {
            values.insert(key, value);
        }
        Ok(values)
    }
}
