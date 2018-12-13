use std::collections::{HashMap, HashSet};

use crate::Graph;

/// A partition of a the nodes of a graph.
///
/// Tarjan's algorithm decomposes a directed graph into strongly connected components.  Moreover,
/// those components are ordered topologically.
pub struct Partition<'a, G: Graph + ?Sized> {
    g: &'a G,
    sets: Vec<HashSet<G::Node>>,
    node_map: HashMap<G::Node, usize>,
}

impl<'a, G: Graph + ?Sized> Partition<'a, G> {
    pub(crate) fn new(g: &'a G, sets: Vec<HashSet<G::Node>>) -> Partition<'a, G> {
        let mut node_map = HashMap::new();
        for (i, component) in sets.iter().enumerate() {
            for u in component {
                node_map.insert(*u, i);
            }
        }
        Partition { g, sets, node_map }
    }

    pub fn num_components(&self) -> usize {
        self.sets.len()
    }

    pub fn parts<'b>(&'b self) -> impl Iterator<Item = &'b HashSet<G::Node>> {
        self.sets.iter()
    }

    pub fn into_parts(self) -> Vec<HashSet<G::Node>> {
        self.sets
    }
}

impl<'a, G: Graph + ?Sized> Graph for Partition<'a, G> {
    type Node = usize;
    type Edge = usize;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = usize>> {
        Box::new(0..self.num_components())
    }

    fn out_edges<'b>(&'b self, u: &usize) -> Box<dyn Iterator<Item = usize>> {
        let mut neighbors = self.sets[*u]
            .iter()
            .flat_map(|u| self.g.out_neighbors(u))
            .map(|v| self.node_map[&v])
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        neighbors.dedup();
        Box::new(neighbors.into_iter())
    }

    fn in_edges<'b>(&'b self, u: &usize) -> Box<dyn Iterator<Item = usize>> {
        let mut neighbors = self.sets[*u]
            .iter()
            .flat_map(|u| self.g.out_neighbors(u))
            .map(|v| self.node_map[&v])
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        neighbors.dedup();
        Box::new(neighbors.into_iter())
    }
}
