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

use itertools::Itertools;
use ojo_graph::Graph;
use ojo_multimap::MMap;
use std::collections::{BTreeMap, HashSet};

use crate::NodeId;

/// A version of a [`Graggle`](crate::Graggle) that has been decomposed into "chains" (for example, for
/// prettier rendering).
///
/// Most graggles that you'll see in practice don't look like general directed graphs. Rather, they
/// contain lots of long "chains," i.e., sequences of nodes, each of which has exactly one
/// in-neighbor (the previous in the sequence) and one out-neighbor (the next). For example, a
/// totally ordered graggle (a.k.a. a file) just consists of one long chain.
///
/// This struct represents the same graph as a graggle, except that every chain has been "collapsed"
/// into a single node. That is, you can think of a `ChainGraggle` as a graph in which every node
/// represents a chain (possibly of length 1) in the original graph.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainGraggle {
    // TODO: allow retrieving liveness of NodeIds and type of edges.
    chains: Vec<Vec<NodeId>>,
    edges: MMap<usize, usize>,
    clusters: Vec<HashSet<usize>>,
}

// Assumes that `node` is not part of a cycle. Therefore, it is on a chain if and only if it
// has at most one in-neighbor and at most one out-neighbor.
fn on_chain<G: Graph>(g: &G, node: &G::Node) -> bool {
    g.out_edges(node).take(2).count() <= 1 && g.in_edges(node).take(2).count() <= 1
}

// Follows a chain backwards to find the first node on it.
fn chain_first<G: Graph>(g: &G, node: &G::Node) -> G::Node {
    if !on_chain(g, node) {
        *node
    } else {
        let mut ret = *node;
        while let Some(prev) = g.in_neighbors(&ret).next() {
            if on_chain(g, &prev) {
                ret = prev;
            } else {
                return ret;
            }
        }
        ret
    }
}

fn collect_chain<G: Graph>(g: &G, first: &G::Node) -> Vec<G::Node> {
    let mut ret = Vec::new();
    let mut cur = *first;
    ret.push(cur);
    if !on_chain(g, &cur) {
        return ret;
    }

    while let Some(next) = g.out_neighbors(&cur).next() {
        if on_chain(g, &next) {
            ret.push(next);
            cur = next;
        } else {
            break;
        }
    }
    ret
}

impl ChainGraggle {
    /// How many chains are there?
    pub fn num_chains(&self) -> usize {
        self.chains.len()
    }

    /// Returns the sequence of `NodeId`s making up the chain at index `i`.
    pub fn chain(&self, i: usize) -> &[NodeId] {
        &self.chains[i]
    }

    /// Returns an iterator over strongly connected components of the original graph.
    pub fn clusters(&self) -> impl Iterator<Item = &HashSet<usize>> {
        self.clusters.iter()
    }

    /// Given a graph, decompose it into a `ChainGraggle`.
    pub fn from_graph<G: Graph<Node = NodeId>>(g: G) -> ChainGraggle
    where
        G::Edge: ojo_graph::Edge<NodeId>,
    {
        let sccs = g.tarjan();

        // The collection of all nodes whose SCC has only one component. These are the ones that
        // can potentially belong to chains.
        let mut singles = sccs
            .parts()
            .filter(|part| part.len() == 1)
            .flat_map(|part| part.iter())
            .collect::<HashSet<_>>();

        // An iterator over nodes in large SCCs.
        let (others1, others2) = sccs
            .parts()
            .filter(|part| part.len() > 1)
            .flat_map(|part| part.iter())
            .cloned()
            .tee();

        // All nodes in larger SCCs get added to the final graph as length-1 chains.
        let mut chains = others1.map(|node| vec![node]).collect::<Vec<_>>();

        // Map going from NodeId to index in the Vec<Node>.
        let mut node_part = others2
            .enumerate()
            .map(|(x, y)| (y, x))
            .collect::<BTreeMap<NodeId, usize>>();

        while !singles.is_empty() {
            let u = chain_first(&g, singles.iter().next().unwrap());
            let chain = collect_chain(&g, &u);
            for v in &chain {
                singles.remove(v);
                node_part.insert(*v, chains.len());
            }
            chains.push(chain);
        }

        let mut edges = MMap::new();

        for u in g.nodes() {
            for v in g.out_neighbors(&u) {
                let u_idx = node_part[&u];
                let v_idx = node_part[&v];

                // Nodes with the same index belong to the same chain, so we don't need to store an
                // edge between them.
                if u_idx != v_idx {
                    edges.insert(u_idx, v_idx);
                }
            }
        }

        let clusters = sccs
            .parts()
            .filter(|part| part.len() > 1)
            .map(|part| part.iter().map(|id| node_part[id]).collect::<HashSet<_>>())
            .collect::<Vec<_>>();

        ChainGraggle {
            chains,
            edges,
            clusters,
        }
    }
}

/// In this implementation of [`graph::Graph`], the nodes are (semantically) chains in the original
/// graggle. More precisely, they are `usize`s that you can use to get the chain with
/// [`ChainGraggle::chain`].
impl Graph for ChainGraggle {
    type Node = usize;
    type Edge = usize;

    fn nodes(&'_ self) -> Box<dyn Iterator<Item = usize> + '_> {
        Box::new(0..self.chains.len())
    }

    fn out_edges(&'_ self, u: &usize) -> Box<dyn Iterator<Item = usize> + '_> {
        Box::new(self.edges.get(u).cloned())
    }

    // TODO: consider removing in_edges from the Graph trait and making it part of a different
    // trait.
    fn in_edges(&'_ self, _u: &usize) -> Box<dyn Iterator<Item = usize> + '_> {
        panic!("in-edges not implemented for this graph");
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{graggle, storage::graggle::tests::arb_live_graggle},
        proptest::prelude::*,
        std::collections::HashSet,
    };

    #[test]
    fn diamond() {
        let graggle = graggle!(
            live: 0, 1, 2, 3
            edges: 0-1, 0-2, 1-3, 2-3
        );
        let decomp = ChainGraggle::from_graph(graggle.as_graggle().as_live_graph());
        assert_eq!(decomp.chains.len(), 4);
        for ch in &decomp.chains {
            assert_eq!(ch.len(), 1);
        }
    }

    proptest! {
        // Checks that the chains of the decomposition form a partition of the original node set.
        #[test]
        fn partition(ref d in arb_live_graggle(20)) {
            let decomp = ChainGraggle::from_graph(d.as_graggle().as_live_graph());
            let decomp_nodes = decomp.chains.iter().flat_map(|chain| chain.iter()).cloned().collect::<Vec<_>>();
            let decomp_node_set = decomp_nodes.iter().cloned().collect::<HashSet<_>>();

            // Check that there are no repeated nodes.
            assert_eq!(decomp_nodes.len(), decomp_node_set.len());

            // Check that we got all the nodes once.
            assert_eq!(decomp_nodes.len(), d.as_graggle().nodes().count());
        }
    }
}
