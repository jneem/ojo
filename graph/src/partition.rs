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

        let mut edges = HashMap::new();
        let mut back_edges = HashMap::new();
        for u in g.nodes() {
            let u_idx = node_map[&u];
            for v in g.out_neighbors(&u) {
                let v_idx = node_map[&v];
                edges.entry(u_idx).or_insert(Vec::new()).push(v_idx);
                back_edges.entry(v_idx).or_insert(Vec::new()).push(u_idx);
            }
        }
        Partition { sets, node_map, edges, back_edges }
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
