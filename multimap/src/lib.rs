extern crate rpds;
extern crate serde;

#[macro_use]
extern crate serde_derive;

use rpds::{RedBlackTreeMap, RedBlackTreeSet};
use std::borrow::Borrow;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MMap<K: Ord, V: Ord> {
    map: RedBlackTreeMap<K, RedBlackTreeSet<V>>,
    // hackity
    empty_set: RedBlackTreeSet<V>,
}

impl<K: Ord, V: Ord> MMap<K, V> {
    pub fn new() -> MMap<K, V> {
        MMap {
            map: RedBlackTreeMap::new(),
            empty_set: RedBlackTreeSet::new(),
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

    pub fn insert(&self, key: K, val: V) -> MMap<K, V> {
        let old_set = self
            .map
            .get(&key)
            .cloned()
            .unwrap_or_else(|| RedBlackTreeSet::new());
        let new_set = old_set.insert(val);
        let new_map = self.map.insert(key, new_set);
        MMap {
            map: new_map,
            empty_set: RedBlackTreeSet::new(),
        }
    }

    // It might seem a bit strange that we need an owned key in order to remove the binding. This
    // is an artifact of our implementation, because it means we effectively need to modify a
    // binding, which actually means we need to create a new binding, which means we need a new key
    // to put in it.
    pub fn remove(&self, key: K, val: &V) -> MMap<K, V> {
        if let Some(old_set) = self.map.get(&key) {
            let new_set = old_set.remove(val);
            MMap {
                map: self.map.insert(key, new_set),
                empty_set: RedBlackTreeSet::new(),
            }
        } else {
            MMap {
                map: self.map.clone(),
                empty_set: RedBlackTreeSet::new(),
            }
        }
    }
}
