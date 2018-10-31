// This is just a hacked-up multimap. Eventually, we'll need to move to a fully persistent (in the
// functional-data-structure sense), on-disk multimap.

#[macro_use]
extern crate serde_derive;

use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

// FIXME: write Deserialize and Serialize manually so as not to expose the implementation.
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

    pub fn remove_all<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.map.remove(key);
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

#[cfg(test)]
mod tests {
    use super::MMap;

    #[test]
    fn get_empty() {
        let mut map = MMap::new();
        assert!(map.get(&1).next().is_none());
        map.insert(1, 2);
        assert!(map.get(&1).next().is_some());
        assert!(map.get(&2).next().is_none());
    }

    #[test]
    fn get_many() {
        let mut map = MMap::new();
        map.insert(1, 2);
        map.insert(1, 3);
        map.insert(1, 2);
        map.insert(1, 1);
        assert_eq!(map.get(&1).cloned().collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn contains() {
        let mut map = MMap::new();
        map.insert(1, 2);
        map.insert(1, 3);
        assert!(map.contains(&1, &2));
        assert!(!map.contains(&2, &1));
        assert!(!map.contains(&1, &4));
    }
}

