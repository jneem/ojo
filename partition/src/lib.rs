//! This crate provides an implementation of the disjoint-sets algorithm that is built on top of
//! a pair of multimaps. (The reason for this weird implementation is that once multimaps is fully
//! persistent, this will be also.)
use multimap::MMap;
use std::collections::{BTreeMap as Map};
use std::collections::btree_map::Entry;

#[derive(Debug)]
pub struct Partition<T: Copy + Ord> {
    ranks: Map<T, usize>,
    parent_map: Map<T, T>,
    child_map: MMap<T, T>,
}

impl<T: Copy + Ord> Partition<T> {
    pub fn new() -> Partition<T> {
        Partition {
            ranks: Map::new(),
            parent_map: Map::new(),
            child_map: MMap::new(),
        }
    }

    /// Panics if the new element already exists.
    pub fn insert(&mut self, elt: T) {
        match self.ranks.entry(elt) {
            Entry::Occupied(_) => panic!("tried to insert an element twice"),
            Entry::Vacant(e) => e.insert(0),
        };
    }

    // Is the given element the representative of its component?
    fn is_rep(&self, elt: &T) -> bool {
        !self.parent_map.contains_key(elt)
    }

    /// Returns true if there was a merge to be done (i.e. they didn't already belong to the same
    /// part).
    pub fn merge(&mut self, elt1: T, elt2: T) -> bool {
        let rep1 = self.representative_mut(elt1);
        let rep2 = self.representative_mut(elt2);
        if rep1 != rep2 {
            self.merge_reps(rep1, rep2);
            true
        } else {
            false
        }
    }

    // Panics unless the two given elements are representatives of their components.
    fn merge_reps(&mut self, rep1: T, rep2: T) {
        assert!(self.is_rep(&rep1) && self.is_rep(&rep2));
        let rank1 = self.ranks[&rep1];
        let rank2 = self.ranks[&rep2];
        if rank1 <= rank2 {
            self.parent_map.insert(rep1, rep2);
            self.child_map.insert(rep2, rep1);
            if rank1 == rank2 {
                self.ranks.insert(rep2, rank2 + 1);
            }
        } else {
            self.parent_map.insert(rep2, rep1);
            self.child_map.insert(rep1, rep2);
        }
    }

    pub fn representative_mut(&mut self, elt: T) -> T {
        let rep = self.representative(elt);
        // Reparent the element to the representative.
        if let Some(orig_parent_ref) = self.parent_map.get_mut(&elt) {
            if *orig_parent_ref != rep {
                self.child_map.remove(&*orig_parent_ref, &elt);
                self.child_map.insert(rep, elt);
                *orig_parent_ref = rep;
            }
        }
        rep
    }

    pub fn representative(&self, elt: T) -> T {
        let mut ret = elt;
        while let Some(parent) = self.parent_map.get(&ret) {
            ret = *parent;
        }
        ret
    }

    pub fn same_part_mut(&mut self, elt1: T, elt2: T) -> bool {
        self.representative_mut(elt1) == self.representative_mut(elt2)
    }

    pub fn same_part(&self, elt1: T, elt2: T) -> bool {
        self.representative(elt1) == self.representative(elt2)
    }

    pub fn remove_part(&mut self, elt: T) {
        let elts = self.iter_part(elt).collect::<Vec<_>>();
        for e in elts {
            self.parent_map.remove(&e);
            self.ranks.remove(&e);
            self.child_map.remove_all(&e);
        }
    }

    pub fn iter_part<'a>(&'a self, elt: T) -> impl Iterator<Item = T> + 'a {
        PartIter::new(self, self.representative(elt))
    }

    pub fn iter_parts<'a>(&'a self) -> impl Iterator<Item = impl Iterator<Item = T> + 'a> + 'a {
        self.ranks
            .keys()
            // For each representative of a part...
            .filter(move |elt| self.is_rep(elt))
            // ...return an iterator over that part.
            .map(move |r| self.iter_part(*r))
    }
}

pub struct PartIter<'a, T: Copy + Ord> {
    partition: &'a Partition<T>,
    // We can traverse a component as though it were a tree, by following the child links. In order
    // to keep track of the iteration we store a stack, each element of which contains an iterator
    // over nodes at a certain level of the tree. Note that each of these iterators is of the type
    // returned by MMap::get; we currently have no way to name this type, hence the Box.
    stack: Vec<Box<dyn Iterator<Item = T> + 'a>>,
}

impl<'a, T: Copy + Ord> PartIter<'a, T> {
    fn new(partition: &'a Partition<T>, root: T) -> PartIter<'a, T> {
        PartIter {
            partition,
            stack: vec![Box::new(Some(root).into_iter())],
        }
    }
}

impl<'a, T: Copy + Ord> Iterator for PartIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(iter) = self.stack.last_mut() {
            if let Some(item) = iter.next() {
                self.stack.push(Box::new(self.partition.child_map.get(&item).cloned()));
                return Some(item);
            } else {
                self.stack.pop();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: think about how to use proptest for testing this
    #[test]
    fn partition() {
        fn assert_vec_eq(mut a: Vec<u32>, mut b: Vec<u32>) {
            a.sort();
            b.sort();
            assert_eq!(a, b);
        }

        let mut partition = Partition::new();
        partition.insert(0);
        partition.insert(1);
        partition.insert(2);
        partition.insert(3);
        partition.insert(4);

        assert_eq!(partition.iter_parts().count(), 5);

        partition.merge(0, 4);
        assert_eq!(partition.iter_parts().count(), 4);
        partition.merge(0, 4);
        assert_eq!(partition.iter_parts().count(), 4);
        assert!(partition.same_part(0, 4));
        assert_vec_eq(partition.iter_part(0).collect(), vec![0, 4]);
        assert_vec_eq(partition.iter_part(4).collect(), vec![0, 4]);

        partition.merge(1, 2);
        assert_eq!(partition.iter_parts().count(), 3);
        assert!(partition.same_part(1, 2));
        assert_vec_eq(partition.iter_part(1).collect(), vec![1, 2]);
        assert_vec_eq(partition.iter_part(2).collect(), vec![1, 2]);

        partition.merge(2, 4);
        assert_eq!(partition.iter_parts().count(), 2);
        assert_vec_eq(partition.iter_part(0).collect(), vec![0, 1, 2, 4]);
        assert_vec_eq(partition.iter_part(1).collect(), vec![0, 1, 2, 4]);
        assert_vec_eq(partition.iter_part(2).collect(), vec![0, 1, 2, 4]);
        assert_vec_eq(partition.iter_part(4).collect(), vec![0, 1, 2, 4]);

        partition.remove_part(1);
        assert_eq!(partition.iter_parts().count(), 1);
        assert_vec_eq(partition.iter_part(3).collect(), vec![3]);
    }
}

