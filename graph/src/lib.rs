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

#[cfg(test)]
#[macro_use]
extern crate proptest;

use {
    itertools::Itertools,
    std::{collections::HashSet, hash::Hash},
};

pub mod dfs;
pub mod partition;
pub mod tarjan;

pub use crate::partition::Partition;

pub trait Edge<N> {
    fn target(&self) -> N;
}

impl<N: Copy> Edge<N> for N {
    fn target(&self) -> N {
        *self
    }
}

pub trait Graph {
    type Node: Copy + Eq + Hash;
    type Edge: Copy + Eq + Edge<Self::Node>;

    // Once impl iterator is available in traits, unbox these.
    fn nodes<'a>(&'a self) -> Box<dyn Iterator<Item = Self::Node> + 'a>;
    fn out_edges<'a>(&'a self, u: &Self::Node) -> Box<dyn Iterator<Item = Self::Edge> + 'a>;
    fn in_edges<'a>(&'a self, u: &Self::Node) -> Box<dyn Iterator<Item = Self::Edge> + 'a>;

    #[expect(clippy::type_complexity)]
    fn out_neighbors<'a>(
        &'a self,
        u: &Self::Node,
    ) -> std::iter::Map<impl Iterator<Item = Self::Edge> + 'a, fn(Self::Edge) -> Self::Node> {
        self.out_edges(u)
            .map((|e| e.target()) as fn(Self::Edge) -> Self::Node)
    }

    #[expect(clippy::type_complexity)]
    fn in_neighbors<'a>(
        &'a self,
        u: &Self::Node,
    ) -> std::iter::Map<impl Iterator<Item = Self::Edge> + 'a, fn(Self::Edge) -> Self::Node> {
        self.in_edges(u)
            .map((|e| e.target()) as fn(Self::Edge) -> Self::Node)
    }

    fn dfs<'a>(&'a self) -> dfs::Dfs<'a, Self> {
        dfs::Dfs::new(self)
    }

    fn dfs_from<'a>(&'a self, root: &Self::Node) -> dfs::Dfs<'a, Self> {
        dfs::Dfs::new_from(self, root)
    }

    fn has_path(&self, u: &Self::Node, v: &Self::Node) -> bool {
        use self::dfs::Visit;

        for visit in self.dfs_from(u) {
            match visit {
                Visit::Edge { dst, .. } if &dst == v => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn tarjan(&self) -> Partition<Self> {
        tarjan::Tarjan::from_graph(self).run()
    }

    fn weak_components(&self) -> Partition<Self> {
        use self::dfs::Visit;

        let mut cur_component: HashSet<Self::Node> = HashSet::new();
        let mut components = Vec::new();
        let doubled = self.doubled();
        for visit in doubled.dfs() {
            match visit {
                Visit::Edge { dst, .. } => {
                    cur_component.insert(dst);
                }
                Visit::Root(u) => {
                    if !cur_component.is_empty() {
                        components.push(cur_component);
                        cur_component = HashSet::new();
                        cur_component.insert(u);
                    } else {
                        cur_component.insert(u);
                    }
                }
                Visit::Retreat { .. } => {}
            }
        }
        if !cur_component.is_empty() {
            components.push(cur_component);
        }
        Partition::new(self, components)
    }

    /// Returns the graph that has edges in both directions for every edge that this graph has in
    /// one direction.
    fn doubled<'a>(&'a self) -> Doubled<'a, Self> {
        Doubled { graph: self }
    }

    /// Returns the subgraph of this graph that is induced by the set of nodes for which
    /// `predicate` returns `true`.
    fn node_filtered<'a, F>(&'a self, predicate: F) -> NodeFiltered<'a, Self, F>
    where
        F: Fn(&Self::Node) -> bool,
    {
        NodeFiltered {
            predicate,
            graph: self,
        }
    }

    /// Returns the subgraph of this graph containing all the edges for which the predicate returns
    /// true.
    fn edge_filtered<'a, F>(&'a self, predicate: F) -> EdgeFiltered<'a, Self, F>
    where
        F: Fn(&Self::Node, &Self::Edge) -> bool,
    {
        EdgeFiltered {
            predicate,
            graph: self,
        }
    }

    /// If this graph is acyclic, returns a topological sort of the vertices. Otherwise, returns
    /// `None`.
    fn top_sort(&self) -> Option<Vec<Self::Node>> {
        use self::dfs::Visit;

        let mut visiting = HashSet::new();
        let mut top_sort = Vec::new();
        // We build up a topological sort in reverse, by running a DFS and adding a node to the
        // topological sort each time we retreat from it.
        for visit in self.dfs() {
            match visit {
                Visit::Edge {
                    src: _,
                    ref dst,
                    status,
                } => {
                    if visiting.contains(dst) {
                        // We found a cycle in the graph, so there is no topological sort.
                        return None;
                    }
                    if status == dfs::Status::New {
                        visiting.insert(*dst);
                    }
                }
                Visit::Retreat { ref u, parent: _ } => {
                    top_sort.push(*u);
                    let removed = visiting.remove(u);
                    assert!(removed);
                }
                Visit::Root(ref u) => {
                    assert!(visiting.is_empty());
                    visiting.insert(*u);
                }
            }
        }
        top_sort.reverse();
        Some(top_sort)
    }

    fn linear_order(&self) -> Option<Vec<Self::Node>> {
        if let Some(top) = self.top_sort() {
            // A graph has a linear order if and only if it has a unique topological sort. A
            // topological sort is unique if and only if every node in it has an edge pointing to
            // the subsequent node.
            for (u, v) in top.iter().tuple_windows() {
                self.out_neighbors(u).position(|x| x == *v)?;
            }
            Some(top)
        } else {
            None
        }
    }

    /// Returns the set of all nodes that are adjacent (either an in-neighbor or an out-neighbor)
    /// to something in `set`.
    fn neighbor_set<'a, I: Iterator<Item = &'a Self::Node>>(&self, set: I) -> HashSet<Self::Node>
    where
        Self::Node: 'a,
    {
        let mut ret = HashSet::new();
        for u in set {
            ret.extend(self.out_neighbors(u));
            ret.extend(self.in_neighbors(u));
        }
        ret
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NodeFiltered<'a, G, F>
where
    G: Graph + ?Sized,
    F: Fn(&G::Node) -> bool + 'a,
{
    predicate: F,
    graph: &'a G,
}

impl<'a, G, F> Graph for NodeFiltered<'a, G, F>
where
    G: Graph + ?Sized,
    F: Fn(&G::Node) -> bool + 'a,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = G::Node> + 'b> {
        Box::new(self.graph.nodes().filter(move |n| (self.predicate)(n)))
    }

    fn out_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        Box::new(
            self.graph
                .out_edges(u)
                .filter(move |e| (self.predicate)(&e.target())),
        )
    }

    fn in_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        Box::new(
            self.graph
                .in_edges(u)
                .filter(move |e| (self.predicate)(&e.target())),
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeFiltered<'a, G, F>
where
    G: Graph + ?Sized,
    F: Fn(&G::Node, &G::Edge) -> bool + 'a,
{
    predicate: F,
    graph: &'a G,
}

impl<'a, G, F> Graph for EdgeFiltered<'a, G, F>
where
    G: Graph + ?Sized,
    F: Fn(&G::Node, &G::Edge) -> bool + 'a,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = G::Node> + 'b> {
        self.graph.nodes()
    }

    fn out_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        let u = *u;
        Box::new(
            self.graph
                .out_edges(&u)
                .filter(move |e| (self.predicate)(&u, e)),
        )
    }

    fn in_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        let u = *u;
        Box::new(
            self.graph
                .in_edges(&u)
                .filter(move |e| (self.predicate)(&u, e)),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Doubled<'a, G: Graph + ?Sized> {
    graph: &'a G,
}

impl<'a, G> Graph for Doubled<'a, G>
where
    G: Graph + ?Sized,
{
    type Node = G::Node;
    type Edge = G::Edge;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = G::Node> + 'b> {
        self.graph.nodes()
    }

    fn out_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        Box::new(self.graph.out_edges(u).chain(self.graph.in_edges(u)))
    }

    fn in_edges<'b>(&'b self, u: &Self::Node) -> Box<dyn Iterator<Item = G::Edge> + 'b> {
        self.out_edges(u)
    }
}

#[cfg(test)]
mod tests {
    use {proptest::prelude::*, std::collections::HashSet};

    use super::Graph;

    #[derive(Clone, Debug)]
    pub struct Node {
        prev: Vec<u32>,
        next: Vec<u32>,
    }

    #[derive(Clone, Debug)]
    pub struct GraphData {
        nodes: Vec<Node>,
        ids: Vec<u32>,
    }

    impl GraphData {
        fn has_edge(&self, u: u32, v: u32) -> bool {
            self.nodes[u as usize].next.contains(&v)
        }
    }

    impl Graph for GraphData {
        type Node = u32;
        type Edge = u32;

        fn nodes<'a>(&'a self) -> Box<dyn Iterator<Item = u32> + 'a> {
            Box::new(self.ids.iter().cloned())
        }

        fn out_edges<'a>(&'a self, u: &u32) -> Box<dyn Iterator<Item = u32> + 'a> {
            Box::new(self.nodes[*u as usize].next.iter().cloned())
        }

        fn in_edges<'a>(&'a self, u: &u32) -> Box<dyn Iterator<Item = u32> + 'a> {
            Box::new(self.nodes[*u as usize].prev.iter().cloned())
        }
    }

    // Given a string like "0-3, 1-2, 3-4, 2-3", creates a graph.
    pub fn graph(s: &str) -> GraphData {
        let mut ret = GraphData {
            nodes: Vec::new(),
            ids: Vec::new(),
        };

        for e in s.split(',') {
            let dash_idx = e.find('-').unwrap();
            let u: usize = e[..dash_idx].trim().parse().unwrap();
            let v: usize = e[(dash_idx + 1)..].trim().parse().unwrap();
            let w = ::std::cmp::max(u, v);

            if w >= ret.nodes.len() {
                let empty_node = Node {
                    next: Vec::new(),
                    prev: Vec::new(),
                };
                ret.ids.extend((ret.ids.len() as u32)..=(w as u32));
                ret.nodes.resize(w + 1, empty_node);
                assert!(ret.ids.len() == ret.nodes.len());
            }

            ret.nodes[u].next.push(v as u32);
            ret.nodes[v].prev.push(u as u32);
        }

        ret
    }

    macro_rules! top_sort_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let top_sort = g.top_sort();
                assert_eq!(top_sort, $expected);
            }
        };
    }

    macro_rules! linear_order_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let order = g.linear_order();
                assert_eq!(order, $expected);
            }
        };
    }

    top_sort_test!(top_sort_chain, "0-1, 1-3, 3-2", Some(vec![0, 1, 3, 2]));
    top_sort_test!(top_sort_cycle, "0-1, 1-2, 2-3, 3-1", None);
    top_sort_test!(top_sort_tree, "0-2, 2-3, 1-3", Some(vec![1, 0, 2, 3]));

    linear_order_test!(linear_order_chain, "0-1, 1-3, 3-2", Some(vec![0, 1, 3, 2]));
    linear_order_test!(
        linear_order_chain_with_extra,
        "0-1, 1-3, 3-2, 0-2",
        Some(vec![0, 1, 3, 2])
    );
    linear_order_test!(
        linear_order_chain_with_extra2,
        "0-1, 0-2, 1-3, 3-2",
        Some(vec![0, 1, 3, 2])
    );
    linear_order_test!(linear_order_cycle, "0-1, 1-2, 2-3, 3-1", None);
    linear_order_test!(linear_order_tree, "0-2, 2-3, 1-3", None);
    linear_order_test!(linear_order_diamond, "0-1, 0-2, 1-3, 2-3", None);

    // A strategy for generating arbitrary graphs (with up to 20 nodes and up to 40 edges).
    prop_compose! {
        [pub(crate)] fn arb_graph()
        (size in 1u32..20)
        (edges in proptest::collection::vec((0..size, 0..size), 1..40), size in Just(size))
        -> GraphData {
            let mut ret = GraphData {
                ids: (0..size).collect(),
                nodes: vec![Node { prev: vec![], next: vec![] }; size as usize],
            };
            for (u, v) in edges {
                ret.nodes[u as usize].next.push(v);
                ret.nodes[v as usize].prev.push(u);
            }
            ret
        }
    }

    // A strategy for generating arbitrary DAGs (with up to 20 nodes and up to 40 edges).
    prop_compose! {
        [pub(crate)] fn arb_dag()
        (size in 1u32..20)
        (edges in proptest::collection::vec((0..size, 0..size), 1..40), size in Just(size))
        -> GraphData {
            let mut ret = GraphData {
                ids: (0..size).collect(),
                nodes: vec![Node { prev: vec![], next: vec![] }; size as usize],
            };
            for (u, v) in edges {
                // We ensure this is a DAG by making sure that the usual ordering from low to high
                // is a topological sort.
                if u < v {
                    ret.nodes[u as usize].next.push(v);
                    ret.nodes[v as usize].prev.push(u);
                } else if v < u {
                    ret.nodes[v as usize].next.push(u);
                    ret.nodes[u as usize].prev.push(v);
                }
            }
            ret
        }
    }

    proptest! {
        #[test]
        fn top_sort_proptest(ref g in arb_graph()) {
            if let Some(sort) = g.top_sort() {
                for i in 0..sort.len() {
                    for j in (i+1)..sort.len() {
                        let u = sort[i];
                        let v = sort[j];
                        // v appears after u in the topological sort, so there must not be any
                        // edge from v to u.
                        assert!(!g.has_edge(v, u));
                    }
                }
            }
        }

        #[test]
        fn doubled_proptest(ref g in arb_graph()) {
            let d = g.doubled();

            // Every edge of the original graph appears in both directions in the doubled graph.
            for u in g.nodes() {
                for v in g.out_neighbors(&u) {
                    assert!(d.out_neighbors(&u).any(|x| x == v));
                    assert!(d.in_neighbors(&u).any(|x| x == v));
                }
            }

            // Every edge of the doubled graph appears in at least one direction in the original.
            for u in d.nodes() {
                for v in d.out_neighbors(&u) {
                    assert!(g.out_neighbors(&u).any(|x| x == v)
                        || g.in_neighbors(&u).any(|x| x == v));
                }
            }
        }

        #[test]
        fn weak_components_proptest(ref g in arb_graph()) {
            // This is not a complete test of the correctness of weak_components: it checks that
            // no two parts of the partition have an edge between them, but it doesn't check that
            // every two elements of a part are weakly connected.
            let partition = g.weak_components();
            for part1 in &partition.sets {
                for part2 in &partition.sets {
                    if part1 != part2 {
                        assert!(part1.is_disjoint(part2));
                        for u in part1 {
                            for v in part2 {
                                assert!(!g.has_edge(*u, *v));
                            }
                        }
                    }
                }
            }

            // Check that every node appears in some component.
            let union = partition.sets.iter().fold(HashSet::new(), |a, b| a.union(b).cloned().collect());
            assert_eq!(g.nodes().collect::<HashSet<_>>(), union);
        }
    }
}
