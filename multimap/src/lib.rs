#[macro_use]
extern crate serde_derive;

use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MMap<K: Ord, V: Ord> {
    map: BTreeMap<K, BTreeSet<V>>,
    // hackity
    empty_set: BTreeSet<V>,
}

impl<K: Ord, V: Ord> MMap<K, V> {
    pub fn new() -> MMap<K, V> {
        MMap {
            map: BTreeMap::new(),
            empty_set: BTreeSet::new(),
        }
    }

    /// Returns an iterator over all the values associated with this key.
    pub fn get<Q>(&self, key: &Q) -> impl Iterator<Item = &V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.map.get(key).unwrap_or(&self.empty_set).iter()
    }

    pub fn insert(&mut self, key: K, val: V) {
        self.map
            .entry(key)
            .or_insert_with(|| BTreeSet::new())
            .insert(val);
    }

    pub fn remove<Q, R>(&mut self, key: &Q, val: &R) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Borrow<R>,
        R: Ord + ?Sized,
    {
        if let Some(set) = self.map.get_mut(&key) {
            set.remove(val)
        } else {
            false
        }
    }

    pub fn contains<Q, R>(&self, key: &Q, val: &R) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
        V: Borrow<R>,
        R: Ord + ?Sized,
    {
        self.map.get(key)
            .and_then(|bindings| bindings.get(val))
            .is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item=(&K, &V)> {
        self.map.iter()
            .flat_map(|(k, vs)| {
                vs.iter().map(move |v| (k, v))
            })
    }
}

// FIXME: tests

