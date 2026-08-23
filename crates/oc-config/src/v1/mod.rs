//! Ported from: packages/core/src/v1/config/* (schema ConfigV1) dan
//! packages/core/src/config/experimental.ts + policy.ts + reference.ts.

pub mod agent;
pub mod attachment;
pub mod command;
pub mod config;
pub mod error;
pub mod experimental;
pub mod formatter;
pub mod layout;
pub mod lsp;
pub mod mcp;
pub mod permission;
pub mod plugin_spec;
pub mod provider;
pub mod reference;
pub mod server;
pub mod skills;

use serde::{Deserialize, Deserializer, Serialize};

/// Ported from: packages/core/schema.ts PositiveInt (via @opencode-ai/schema)
pub fn positive_int<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u64::deserialize(deserializer)?;
    if value == 0 {
        return Err(serde::de::Error::custom("Expected positive integer"));
    }
    Ok(value)
}

/// Ported from: packages/core/schema.ts NonNegativeInt
pub fn non_negative_int<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    if value < 0 {
        return Err(serde::de::Error::custom("Expected non-negative integer"));
    }
    Ok(value as u64)
}

/// Varian Option untuk field opsional bertipe NonNegativeInt.
pub fn non_negative_int_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<i64>::deserialize(deserializer)?;
    match value {
        Some(v) if v < 0 => Err(serde::de::Error::custom("Expected non-negative integer")),
        other => Ok(other.map(|v| v as u64)),
    }
}

/// Map string→T yang order-preserving (meniru objek JS). Dipakai karena
/// `serde_json::Map` hanya mengimplementasikan trait untuk `Value`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedMap<T> {
    pub entries: Vec<(String, T)>,
}

impl<T> OrderedMap<T> {
    pub fn new() -> Self {
        OrderedMap {
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: T) -> Option<T> {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            return Some(std::mem::replace(&mut slot.1, value));
        }
        self.entries.push((key, value));
        None
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Meniru `Object.assign(target, source)` — hanya key yang ada di sumber.
    pub fn assign_from(&mut self, other: OrderedMap<T>) {
        for (key, value) in other.entries {
            self.insert(key, value);
        }
    }
}

impl<T> FromIterator<(String, T)> for OrderedMap<T> {
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        let mut map = OrderedMap::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<T: Serialize> Serialize for OrderedMap<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.entries.iter().map(|(k, v)| (k, v)))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for OrderedMap<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = OrderedMap<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut map = OrderedMap::new();
                while let Some((key, value)) = access.next_entry()? {
                    map.insert(key, value);
                }
                Ok(map)
            }
        }

        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
