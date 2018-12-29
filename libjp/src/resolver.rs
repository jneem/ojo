use graph::Graph;
use std::collections::{HashMap, HashSet};

use crate::{Change, Changes, Digle, NodeId};

pub struct CycleResolver<'a> {
    digle: Digle<'a>,
    sccs: graph::Partition<Digle<'a>>,

    // The indices of all SCCs that have more than one element. This will gradually shrink as we
    // resolve more components.
    large_sccs: Vec<usize>,

    // For the components that have already been resolved, this contains the representatives that
    // were chosen to live.
    scc_reps: HashMap<usize, NodeId>,
}

impl<'a> CycleResolver<'a> {
    pub fn new(digle: Digle<'a>) -> CycleResolver<'a> {
        let sccs = digle.tarjan();
        let large_sccs = sccs
            .parts()
            .enumerate()
            .filter(|(_, part)| part.len() >= 2)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        CycleResolver {
            digle,
            sccs,
            large_sccs,
            scc_reps: HashMap::new(),
        }
    }

    pub fn next_component(&self) -> Option<&HashSet<NodeId>> {
        self.large_sccs.last().map(|i| self.sccs.part(*i))
    }

    // Which component are we currently working on?
    //
    // Panics if we are finished.
    fn cur(&self) -> usize {
        *self.large_sccs.last().unwrap()
    }

    pub fn resolve_component(&mut self, rep: NodeId) {
        assert!(self.sccs.part(self.cur()).contains(&rep));
        let cur = self.large_sccs.pop().unwrap();
        self.scc_reps.insert(cur, rep);
    }

    pub fn into_order_resolver(self) -> OrderResolver<'a> {
        assert!(self.large_sccs.is_empty());

        let scc_reps = (0..self.sccs.num_components())
            .map(|i| {
                if let Some(&u) = self.scc_reps.get(&i) {
                    u
                } else {
                    // If we haven't explicitly found a representative for this component, it must
                    // have originally been a component of size 1.
                    let mut iter = self.sccs.part(i).iter();
                    let rep = iter.next().expect("components must be non-empty");
                    assert!(iter.next().is_none(), "this component must have size 1");
                    *rep
                }
            })
            .collect::<Vec<_>>();

        let in_edge_count = self
            .sccs
            .nodes()
            .map(|u| (u, self.sccs.in_edges(&u).count()))
            .collect::<HashMap<_, _>>();
        let candidates = in_edge_count
            .iter()
            .filter(|&(_, &count)| count == 0)
            .map(|(u, _)| *u)
            .collect::<HashSet<_>>();

        OrderResolver {
            digle: self.digle,
            ordered: vec![],
            seen: HashSet::new(),
            sccs: self.sccs,
            scc_reps,
            remaining_in_edges: in_edge_count,
            candidates,
        }
    }
}

pub struct OrderResolver<'a> {
    digle: Digle<'a>,
    ordered: Vec<NodeId>,

    // The partition of the digle's nodes into strongly connected components. All of the remaining
    // fields refer to indices of components in this partition.
    sccs: graph::Partition<Digle<'a>>,
    // Since OrderResolver comes after CycleResolver, we have already chosen exactly one
    // representative from each SCC. This is the list of representatives.
    scc_reps: Vec<NodeId>,

    seen: HashSet<usize>,
    candidates: HashSet<usize>,
    remaining_in_edges: HashMap<usize, usize>,
}

impl<'a> OrderResolver<'a> {
    pub fn ordered_nodes(&self) -> &[NodeId] {
        &self.ordered[..]
    }

    pub fn candidates<'b>(
        &'b self,
    ) -> impl Iterator<Item = impl Iterator<Item = NodeId> + 'a> + 'b {
        self.candidates
            .iter()
            .map(move |&i| ChainIter::new(self.digle, self.scc_reps[i]))
    }

    fn advance_past(&mut self, node_idx: usize) {
        self.candidates.remove(&node_idx);
        for u in self.sccs.out_neighbors(&node_idx) {
            // The unwrap is ok because remaining_in_edges contains every node as a key.
            let remaining = self.remaining_in_edges.get_mut(&u).unwrap();
            assert!(*remaining >= 1);
            *remaining -= 1;
            if *remaining == 0 {
                self.candidates.insert(u);
            }
        }
    }

    pub fn choose(&mut self, next: &NodeId) {
        let next_idx = self.sccs.index_of(next);
        assert!(self.candidates.contains(&next_idx));

        self.ordered.push(*next);
        self.seen.insert(next_idx);

        self.advance_past(next_idx);
    }

    pub fn delete(&mut self, u: &NodeId) {
        let u_idx = self.sccs.index_of(u);
        assert!(self.candidates.contains(&u_idx));
        self.advance_past(u_idx);
    }

    // TODO:
    // pub fn insert(&mut self, ...)

    pub fn is_finished(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn changes(&self) -> Changes {
        let mut changes = vec![];

        // All nodes that didn't make it to the final output should be marked as deleted.
        let not_deleted = self.ordered.iter().cloned().collect::<HashSet<_>>();
        for u in self.digle.nodes() {
            if !not_deleted.contains(&u) {
                changes.push(Change::DeleteNode { id: u });
            }
        }

        // Add all edges that are needed to enforce the linear order.
        for i in 1..self.ordered.len() {
            let u = self.ordered[i - 1];
            let v = self.ordered[i];
            if !self.digle.out_neighbors(&u).any(|w| *w == v) {
                changes.push(Change::NewEdge { src: u, dst: v });
            }
        }

        // TODO: once we allow insertions, add those changes too.

        Changes { changes }
    }
}

struct ChainIter<'a> {
    next: Option<NodeId>,
    digle: Digle<'a>,
}

impl<'a> ChainIter<'a> {
    fn new(digle: Digle<'a>, u: NodeId) -> ChainIter<'a> {
        ChainIter {
            next: Some(u),
            digle: digle,
        }
    }
}

impl<'a> Iterator for ChainIter<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let ret = self.next;

        if let Some(cur) = self.next {
            self.next = None;

            let mut neighbors = self.digle.out_neighbors(&cur);
            if let Some(next) = neighbors.next() {
                let mut next_in = self.digle.in_neighbors(&next);

                // We want to continue iterating if and only if cur has exactly one out-neighbor and
                // that out-neighbor has exactly one in-neighbor.
                if neighbors.next().is_none()
                    && next_in.next().is_some()
                    && next_in.next().is_none()
                {
                    self.next = Some(*next);
                }
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    use crate::storage::digle::tests::make_digle;

    #[test]
    fn chain_iter() {
        let digle = make_digle("0-1, 1-2, 2-3, 3-4, 4-5, 0-5, 2-5");
        let check = |init: u64, expected: Vec<u64>| {
            let actual = ChainIter::new(digle.as_digle(), NodeId::cur(init)).collect::<Vec<_>>();
            let expected = expected
                .into_iter()
                .map(|x| NodeId::cur(x))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        };
        check(0, vec![0]);
        check(1, vec![1, 2]);
        check(2, vec![2]);
        check(3, vec![3, 4]);
        check(4, vec![4]);
        check(5, vec![5]);
    }

    #[test]
    fn resolver_diamond() {
        let digle = make_digle("0-1, 0-2, 1-3, 2-3");
        let mut res = CycleResolver::new(digle.as_digle()).into_order_resolver();

        println!("{:?}", res.candidates);
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().collect::<Vec<_>>(),
            vec![NodeId::cur(0)]
        );

        res.choose(&NodeId::cur(0));
        assert_eq!(res.candidates().count(), 2);
        assert_eq!(
            res.candidates().flatten().sorted().collect::<Vec<_>>(),
            vec![NodeId::cur(1), NodeId::cur(2)]
        );

        res.choose(&NodeId::cur(1));
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().collect::<Vec<_>>(),
            vec![NodeId::cur(2)]
        );

        res.choose(&NodeId::cur(2));
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().collect::<Vec<_>>(),
            vec![NodeId::cur(3)]
        );

        res.choose(&NodeId::cur(3));
        assert_eq!(res.candidates().count(), 0);

        assert_eq!(
            res.changes(),
            Changes {
                changes: vec![Change::NewEdge {
                    src: NodeId::cur(1),
                    dst: NodeId::cur(2)
                }]
            }
        );
    }
}
