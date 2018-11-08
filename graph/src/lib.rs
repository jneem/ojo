use itertools::Itertools;
use std::collections::HashSet;
use std::hash::Hash;

pub mod dfs;
pub mod tarjan;

pub trait Edge<N> {
    fn target(&self) -> N;
}

impl<N: Copy> Edge<N> for N {
    fn target(&self) -> N {
        *self
    }
}

pub trait Graph<'a>: 'a
{
    type Node: Copy + Eq + Hash;
    type Edge: Copy + Eq + Edge<Self::Node>;
    type NodesIter: Iterator<Item = Self::Node> + 'a;
    type EdgesIter: Iterator<Item = Self::Edge> + 'a;

    fn nodes(&'a self) -> Self::NodesIter;
    fn out_edges(&'a self, u: &Self::Node) -> Self::EdgesIter;
    fn in_edges(&'a self, u: &Self::Node) -> Self::EdgesIter;

    fn out_neighbors(&'a self, u: &Self::Node) -> std::iter::Map<Self::EdgesIter, fn(Self::Edge) -> Self::Node> {
        self.out_edges(u).map((|e| e.target()) as fn(Self::Edge) -> Self::Node)
    }

    fn in_neighbors(&'a self, u: &Self::Node) -> std::iter::Map<Self::EdgesIter, fn(Self::Edge) -> Self::Node> {
        self.in_edges(u).map((|e| e.target()) as fn(Self::Edge) -> Self::Node)
    }

    fn dfs(&'a self) -> dfs::Dfs<'a, Self> {
        dfs::Dfs::new(self)
    }

    fn tarjan(&'a self) -> tarjan::Decomposition<'a, Self> {
        tarjan::Decomposition::from_graph(self)
    }

    /// If this graph is acyclic, returns a topological sort of the vertices. Otherwise, returns
    /// `None`.
    fn top_sort(&'a self) -> Option<Vec<Self::Node>> {
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
                        visiting.insert(dst.clone());
                    }
                }
                Visit::Retreat { ref u, parent: _ } => {
                    top_sort.push(u.clone());
                    let removed = visiting.remove(u);
                    assert!(removed);
                }
                Visit::Root(ref u) => {
                    assert!(visiting.is_empty());
                    visiting.insert(u.clone());
                }
            }
        }
        top_sort.reverse();
        Some(top_sort)
    }

    fn linear_order(&'a self) -> Option<Vec<Self::Node>> {
        if let Some(top) = self.top_sort() {
            // A graph has a linear order if and only if it has a unique topological sort. A
            // topological sort is unique if and only if every node in it has an edge pointing to
            // the subsequent node.
            for (u, v) in top.iter().tuples() {
                if self.out_neighbors(u).position(|x| x == *v).is_none() {
                    return None;
                }
            }
            Some(top)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NodeFiltered<'a, G, F>
where
    G: Graph<'a>,
    F: Fn(&G::Node) -> bool + 'a,
{
    predicate: F,
    graph: G,
    marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, G, F> Graph<'a> for NodeFiltered<'a, G, F> 
where
    G: Graph<'a>,
    F: Fn(&G::Node) -> bool + 'a,
{
    // TODO: unbox this once there is the appropriate support for impl trait
    type Node = G::Node;
    type Edge = G::Edge;
    type NodesIter = Box<Iterator<Item = Self::Node> + 'a>;
    type EdgesIter = Box<Iterator<Item = Self::Edge> + 'a>;

    fn nodes(&'a self) -> Self::NodesIter {
        Box::new(self.graph.nodes().filter(move |n| (self.predicate)(n)))
    }

    fn out_edges(&'a self, u: &Self::Node) -> Self::EdgesIter {
        Box::new(self.graph.out_edges(u).filter(move |e| (self.predicate)(&e.target())))
    }

    fn in_edges(&'a self, u: &Self::Node) -> Self::EdgesIter {
        Box::new(self.graph.in_edges(u).filter(move |e| (self.predicate)(&e.target())))
    }
}

#[cfg(test)]
mod tests {
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

    impl<'a> Graph<'a> for GraphData {
        type Node = u32;
        type Edge = u32;
        type NodesIter = std::iter::Cloned<std::slice::Iter<'a, u32>>;
        type EdgesIter = std::iter::Cloned<std::slice::Iter<'a, u32>>;

        fn nodes(&'a self) -> Self::NodesIter {
            self.ids.iter().cloned()
        }

        fn out_edges(&'a self, u: &u32) -> Self::EdgesIter {
            self.nodes[*u as usize].next.iter().cloned()
        }

        fn in_edges(&'a self, u: &u32) -> Self::EdgesIter {
            self.nodes[*u as usize].prev.iter().cloned()
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
}
