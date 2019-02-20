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

use std::collections::{HashMap, HashSet};

use crate::Graph;

/// A partition of a the nodes of a graph.
///
/// Tarjan's algorithm decomposes a directed graph into strongly connected components.  Moreover,
/// those components are ordered topologically.
pub struct Partition<G: Graph + ?Sized> {
    pub(crate) sets: Vec<HashSet<G::Node>>,
    node_map: HashMap<G::Node, usize>,
    edges: HashMap<usize, Vec<usize>>,
    back_edges: HashMap<usize, Vec<usize>>,
}

impl<G: Graph + ?Sized> Partition<G> {
    pub(crate) fn new(g: &G, sets: Vec<HashSet<G::Node>>) -> Partition<G> {
        let mut node_map = HashMap::new();
        for (i, component) in sets.iter().enumerate() {
            for u in component {
                node_map.insert(*u, i);
            }
        }

        let mut edges = (0..sets.len())
            .map(|u| (u, Vec::new()))
            .collect::<HashMap<_, _>>();
        let mut back_edges = (0..sets.len())
            .map(|u| (u, Vec::new()))
            .collect::<HashMap<_, _>>();
        for u in g.nodes() {
            let u_idx = node_map[&u];
            for v in g.out_neighbors(&u) {
                let v_idx = node_map[&v];

                if u_idx != v_idx {
                    edges.get_mut(&u_idx).unwrap().push(v_idx);
                    back_edges.get_mut(&v_idx).unwrap().push(u_idx);
                }
            }
        }
        Partition {
            sets,
            node_map,
            edges,
            back_edges,
        }
    }

    pub fn num_components(&self) -> usize {
        self.sets.len()
    }

    pub fn parts<'b>(&'b self) -> impl Iterator<Item = &'b HashSet<G::Node>> {
        self.sets.iter()
    }

    pub fn part(&self, i: usize) -> &HashSet<G::Node> {
        &self.sets[i]
    }

    pub fn index_of(&self, u: &G::Node) -> usize {
        self.node_map[&u]
    }

    pub fn into_parts(self) -> Vec<HashSet<G::Node>> {
        self.sets
    }
}

impl<G: Graph + ?Sized> Graph for Partition<G> {
    type Node = usize;
    type Edge = usize;

    fn nodes<'a>(&'a self) -> Box<dyn Iterator<Item = usize>> {
        Box::new(0..self.num_components())
    }

    fn out_edges<'a>(&'a self, u: &usize) -> Box<dyn Iterator<Item = usize> + 'a> {
        Box::new(self.edges[&*u].iter().cloned())
    }

    fn in_edges<'a>(&'a self, u: &usize) -> Box<dyn Iterator<Item = usize> + 'a> {
        Box::new(self.back_edges[&*u].iter().cloned())
    }
}
