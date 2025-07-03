// Copyright 2018-2019 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// See the LICENSE-APACHE or LICENSE-MIT files at the top-level directory
// of this distribution.

//! This module implements some tools that can be used to create interactive tools for resolving
//! non-linearly-ordered graggles into linearly-ordered files.
//!
//! There are essentially two reasons that a graggle can fail to be linearly ordered: it can have
//! cycles (i.e. too many edges) or it can have nodes with no prescribed edge between them (i.e.
//! too few edges). The tools here implement a two-stage process: first, we deal with any cycles
//! using [`CycleResolver`](crate::resolver::CycleResolver); then, we add any necessary edges using
//! [`OrderResolver`](crate::resolver::OrderResolver).

use {
    ojo_graph::Graph,
    std::collections::{HashMap, HashSet},
};

use crate::{Change, Changes, Graggle, LiveGraph, NodeId};

// TODO: implement undo

/// A utility for interactively removing cycles from a graggle.
///
/// Since you can never actually delete edges from a graggle, cycles are resolved by deleting nodes.
/// Specifically, we divide a graggle into its strongly connected components. From each strongly
/// connected component, you must select exactly one node to survive.
pub struct CycleResolver<'a> {
    graggle: Graggle<'a>,
    sccs: ojo_graph::Partition<LiveGraph<'a>>,

    // The indices of all SCCs that have more than one element. This will gradually shrink as we
    // resolve more components.
    large_sccs: Vec<usize>,

    // For the components that have already been resolved, this contains the representatives that
    // were chosen to live.
    scc_reps: HashMap<usize, NodeId>,
}

impl<'a> CycleResolver<'a> {
    /// Creates a new resolver for eliminating cycles in the given graggle.
    pub fn new(graggle: Graggle<'a>) -> CycleResolver<'a> {
        let sccs = graggle.as_live_graph().tarjan();
        let large_sccs = sccs
            .parts()
            .enumerate()
            .filter(|(_, part)| part.len() >= 2)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        CycleResolver {
            graggle,
            sccs,
            large_sccs,
            scc_reps: HashMap::new(),
        }
    }

    /// If there are any strongly connected components remaining, returns the next one that needs
    /// to be resolved.
    pub fn next_component(&self) -> Option<&HashSet<NodeId>> {
        self.large_sccs.last().map(|i| self.sccs.part(*i))
    }

    // Which component are we currently working on?
    //
    // Panics if we are finished.
    fn cur(&self) -> usize {
        *self.large_sccs.last().unwrap()
    }

    /// Resolves the current strongly connected component by deleting all nodes in it except for
    /// `rep`.
    ///
    /// # Panics
    ///
    /// Panics unless `rep` is an element of the current component (as returned by
    /// [`next_component`](CycleResolver::next_component)).
    pub fn resolve_component(&mut self, rep: NodeId) {
        assert!(self.sccs.part(self.cur()).contains(&rep));
        let cur = self.large_sccs.pop().unwrap();
        self.scc_reps.insert(cur, rep);
    }

    /// Assuming that all cycles have already been taken care of, moves to the next stage of
    /// resolution.
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
        // Is there a natural order to put the candidates in?
        let candidates = in_edge_count
            .iter()
            .filter(|&(_, &count)| count == 0)
            .map(|(u, _)| *u)
            .collect::<Vec<_>>();

        OrderResolver {
            graggle: self.graggle,
            ordered: vec![],
            seen: HashSet::new(),
            sccs: self.sccs,
            scc_reps,
            remaining_in_edges: in_edge_count,
            candidates,
        }
    }
}

/// A sequence of nodes that might come next in the file.
///
/// While interactively resolving the order of a file, there could be several choices for the next
/// node (let's call these candidates). Moreover, each candidate might have a sequence of nodes
/// that naturally (but don't necessarily) come after it. This candidate, plus the sequence of
/// nodes that follow it, make up a `CandidateChain`.
///
/// For example, suppose we have a graggle like this:
///
/// ```text
///    -> B -> C -> D
///  /               \
/// A                 -> H
///  \               /
///    -> E -> F -> G
/// ```
///
/// If `A` has already been chosen then `B` would be the head of a candidate chain containing `B`,
/// `C`, and `D`.
pub struct CandidateChain<'a> {
    graggle: Graggle<'a>,
    id: NodeId,
}

impl<'a> CandidateChain<'a> {
    /// Returns the first element of this chain.
    pub fn first(&self) -> NodeId {
        self.id
    }

    /// Returns an iterator over all elements of this chain (including the first).
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + 'a + use<'a> {
        ChainIter::new(self.graggle, self.id)
    }
}

/// A utility for interactively imposing a linear order on a graggle with no cycles.
///
/// You will usually create this struct using [`CycleResolver::into_order_resolver`],
/// which will ensure that there are no cycles remaining.
pub struct OrderResolver<'a> {
    graggle: Graggle<'a>,
    ordered: Vec<NodeId>,

    // The partition of the graggle's nodes into strongly connected components. All of the remaining
    // fields refer to indices of components in this partition.
    sccs: ojo_graph::Partition<LiveGraph<'a>>,
    // Since OrderResolver comes after CycleResolver, we have already chosen exactly one
    // representative from each SCC. This is the list of representatives.
    scc_reps: Vec<NodeId>,

    seen: HashSet<usize>,
    candidates: Vec<usize>,
    remaining_in_edges: HashMap<usize, usize>,
}

impl<'a> OrderResolver<'a> {
    /// Returns a slice containing the nodes that have already been put in order.
    pub fn ordered_nodes(&self) -> &[NodeId] {
        &self.ordered[..]
    }

    /// Returns an iterator over the current set of candidates.
    ///
    /// Each of the returned values represents a node (or sequence of nodes) that could go next in
    /// the output.
    pub fn candidates<'b>(&'b self) -> impl Iterator<Item = CandidateChain<'a>> + 'b {
        self.candidates.iter().map(move |u| CandidateChain {
            graggle: self.graggle,
            id: self.scc_reps[*u],
        })
    }

    fn advance_past(&mut self, scc: usize) {
        // We're removing a candidate, and potentially adding some more. For continuity in the
        // user-interface, we insert the new candidates in the same position as the old ones. This
        // could be made more efficient, but it's probably mostly ok because the list of candidates
        // should be short.
        let idx = self
            .candidates
            .iter()
            .position(|x| *x == scc)
            .expect("tried to remove a non-candidate");
        self.candidates.remove(idx);
        for u in self.sccs.out_neighbors(&scc) {
            // The unwrap is ok because remaining_in_edges contains every node as a key.
            let remaining = self.remaining_in_edges.get_mut(&u).unwrap();
            assert!(*remaining >= 1);
            *remaining -= 1;
            if *remaining == 0 {
                self.candidates.insert(idx, u);
            }
        }
    }

    /// Chooses a node to go next in the ordered output.
    ///
    /// The chosen node must be a valid choice, meaning that there cannot be an edge from the
    /// chosen node to a node that has not yet been chosen. If the chosen node was taken from the
    /// head of one of the chains returned by [`OrderResolver::candidates`], it is guaranteed to be
    /// a valid choice.
    ///
    /// # Panics
    ///
    /// Panics if the chosen node is not a valid choice.
    pub fn choose(&mut self, next: &NodeId) {
        let next_idx = self.sccs.index_of(next);
        assert!(self.candidates.contains(&next_idx));

        self.ordered.push(*next);
        self.seen.insert(next_idx);

        self.advance_past(next_idx);
    }

    /// Deletes a node, instead of including it in the ordered output.
    ///
    /// The chosen node must be valid in the sense described in [`OrderResolver::choose`].
    ///
    /// # Panics
    ///
    /// Panics if the chosen node is not a valid choice.
    pub fn delete(&mut self, u: &NodeId) {
        let u_idx = self.sccs.index_of(u);
        assert!(self.candidates.contains(&u_idx));
        self.advance_past(u_idx);
    }

    // TODO:
    // pub fn insert(&mut self, ...)

    /// Returns true if the entire graggle has already been put in order.
    pub fn is_finished(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Assuming that the entire graggle has already been put in order, returns a [`Changes`] that,
    /// when applied to the graggle, will turn it from the original graggle into the linear order that
    /// we have just created (and which can be retrieved by [`OrderResolver::ordered_nodes`]).
    pub fn changes(&self) -> Changes {
        let mut changes = vec![];

        // All nodes that didn't make it to the final output should be marked as deleted.
        let not_deleted = self.ordered.iter().cloned().collect::<HashSet<_>>();
        for u in self.graggle.nodes() {
            if !not_deleted.contains(&u) {
                changes.push(Change::DeleteNode { id: u });
            }
        }

        // Add all edges that are needed to enforce the linear order.
        for i in 1..self.ordered.len() {
            let u = self.ordered[i - 1];
            let v = self.ordered[i];
            if !self.graggle.out_neighbors(&u).any(|w| *w == v) {
                changes.push(Change::NewEdge { src: u, dest: v });
            }
        }

        // TODO: once we allow insertions, add those changes too.

        Changes { changes }
    }
}

struct ChainIter<'a> {
    next: Option<NodeId>,
    graggle: Graggle<'a>,
}

impl<'a> ChainIter<'a> {
    fn new(graggle: Graggle<'a>, u: NodeId) -> ChainIter<'a> {
        ChainIter {
            next: Some(u),
            graggle,
        }
    }
}

impl<'a> Iterator for ChainIter<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let ret = self.next;

        if let Some(cur) = self.next {
            self.next = None;

            let mut neighbors = self.graggle.out_neighbors(&cur);
            if let Some(next) = neighbors.next() {
                let mut next_in = self.graggle.in_neighbors(next);

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

    #[test]
    fn chain_iter() {
        let graggle = graggle!(
            live: 0, 1, 2, 3, 4, 5
            edges: 0-1, 1-2, 2-3, 3-4, 4-5, 0-5, 2-5
        );
        let check = |init: u64, expected: Vec<u64>| {
            let actual =
                ChainIter::new(graggle.as_graggle(), NodeId::cur(init)).collect::<Vec<_>>();
            let expected = expected.into_iter().map(NodeId::cur).collect::<Vec<_>>();
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
        let graggle = graggle!(
            live: 0, 1, 2, 3
            edges: 0-1, 0-2, 1-3, 2-3
        );
        let mut res = CycleResolver::new(graggle.as_graggle()).into_order_resolver();

        println!("{:?}", res.candidates);
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().iter().collect::<Vec<_>>(),
            vec![NodeId::cur(0)]
        );

        res.choose(&NodeId::cur(0));
        assert_eq!(res.candidates().count(), 2);
        assert_eq!(
            res.candidates()
                .flat_map(|x| x.iter())
                .sorted()
                .collect::<Vec<_>>(),
            vec![NodeId::cur(1), NodeId::cur(2)]
        );

        res.choose(&NodeId::cur(1));
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().iter().collect::<Vec<_>>(),
            vec![NodeId::cur(2)]
        );

        res.choose(&NodeId::cur(2));
        assert_eq!(res.candidates().count(), 1);
        assert_eq!(
            res.candidates().next().unwrap().iter().collect::<Vec<_>>(),
            vec![NodeId::cur(3)]
        );

        res.choose(&NodeId::cur(3));
        assert_eq!(res.candidates().count(), 0);

        assert_eq!(
            res.changes(),
            Changes {
                changes: vec![Change::NewEdge {
                    src: NodeId::cur(1),
                    dest: NodeId::cur(2)
                }]
            }
        );
    }
}
