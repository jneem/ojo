use itertools::Itertools;
use std::collections::{HashMap, HashSet};

use LineId;

pub mod dfs;
pub mod tarjan;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigleNode {
    contents: Vec<u8>,
    //alive: bool,
    prev: Vec<LineId>,
    next: Vec<LineId>,
    //prev_alive: Vec<usize>,
    //next_alive: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Digle {
    nodes: HashMap<LineId, DigleNode>,
    //alive_indices: Vec<usize>,
}

impl Digle {
    pub fn add_node(&mut self, id: &LineId, contents: Vec<u8>) {
        assert!(!self.nodes.contains_key(id));

        let node = DigleNode {
            contents: contents,
            prev: Vec::new(),
            next: Vec::new(),
        };
        self.nodes.insert(id.clone(), node);
    }

    pub fn add_edge(&mut self, from: &LineId, to: &LineId) {
        assert!(self.nodes.contains_key(from));
        assert!(self.nodes.contains_key(to));

        self.nodes.get_mut(from).unwrap().next.push(to.clone());
        self.nodes.get_mut(to).unwrap().prev.push(from.clone());
    }

    pub fn new() -> Digle {
        Digle {
            nodes: HashMap::new(),
        }
    }
}

pub trait GraphRef<'a>: Copy + 'a {
    type NodesIter: Iterator<Item = &'a LineId>;
    type OutNeighborsIter: Iterator<Item = &'a LineId>;
    type InNeighborsIter: Iterator<Item = &'a LineId>;

    fn nodes(self) -> Self::NodesIter;
    fn out_neighbors(self, u: &LineId) -> Self::OutNeighborsIter;
    fn in_neighbors(self, u: &LineId) -> Self::InNeighborsIter;

    fn dfs(self) -> dfs::Dfs<'a, Self> {
        dfs::Dfs::new(self)
    }

    fn tarjan(self) -> tarjan::Decomposition {
        tarjan::Decomposition::from_graph(self)
    }

    /// If this graph is acyclic, returns a topological sort of the vertices. Otherwise, returns
    /// `None`.
    fn top_sort(self) -> Option<Vec<LineId>> {
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

    fn linear_order(self) -> Option<Vec<LineId>> {
        if let Some(top) = self.top_sort() {
            // A graph has a linear order if and only if it has a unique topological sort. A
            // topological sort is unique if and only if every node in it has an edge pointing to
            // the subsequent node.
            for (u, v) in top.iter().tuples() {
                if self.out_neighbors(u).position(|x| x == v).is_none() {
                    return None;
                }
            }
            Some(top)
        } else {
            None
        }
    }
}

impl<'a> GraphRef<'a> for &'a Digle {
    type NodesIter = ::std::collections::hash_map::Keys<'a, LineId, DigleNode>;
    type OutNeighborsIter = ::std::slice::Iter<'a, LineId>;
    type InNeighborsIter = ::std::slice::Iter<'a, LineId>;

    fn nodes(self) -> Self::NodesIter {
        self.nodes.keys()
    }

    fn out_neighbors(self, u: &LineId) -> Self::OutNeighborsIter {
        self.nodes[u].next.iter()
    }

    fn in_neighbors(self, u: &LineId) -> Self::InNeighborsIter {
        self.nodes[u].prev.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{Digle, DigleNode, GraphRef};
    use std::collections::HashMap;
    use {LineId, PatchId};

    #[derive(Clone, Debug)]
    pub struct Node {
        prev: Vec<LineId>,
        next: Vec<LineId>,
    }

    #[derive(Clone, Debug)]
    pub struct Graph {
        nodes: Vec<Node>,
        ids: Vec<LineId>,
    }

    impl<'a> GraphRef<'a> for &'a Graph {
        type NodesIter = ::std::slice::Iter<'a, LineId>;
        type OutNeighborsIter = ::std::slice::Iter<'a, LineId>;
        type InNeighborsIter = ::std::slice::Iter<'a, LineId>;

        fn nodes(self) -> Self::NodesIter {
            self.ids.iter()
        }

        fn out_neighbors(self, u: &LineId) -> Self::OutNeighborsIter {
            self.nodes[u.line as usize].next.iter()
        }

        fn in_neighbors(self, u: &LineId) -> Self::InNeighborsIter {
            self.nodes[u.line as usize].prev.iter()
        }
    }

    // Given a string like "0-3, 1-2, 3-4, 2-3", creates a graph.
    pub fn graph(s: &str) -> Graph {
        let mut ret = Graph {
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
                ret.ids
                    .extend((ret.ids.len()..(w + 1)).map(|x| id(x as u64)));
                ret.nodes.resize(w + 1, empty_node);
                assert!(ret.ids.len() == ret.nodes.len());
            }

            ret.nodes[u].next.push(id(v as u64));
            ret.nodes[v].prev.push(id(u as u64));
        }

        ret
    }

    pub fn id(n: u64) -> LineId {
        LineId {
            patch: PatchId::cur(),
            line: n,
        }
    }

    // Given an array of numbers, creates a matching vec of LineIds.
    pub fn ids(nums: &[u64]) -> Vec<LineId> {
        nums.into_iter().cloned().map(id).collect()
    }

    macro_rules! top_sort_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let top_sort = g.top_sort();
                let expected = $expected.map(|nums: Vec<u64>| ids(&nums));
                assert_eq!(top_sort, expected);
            }
        };
    }

    macro_rules! linear_order_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let order = g.linear_order();
                let expected = $expected.map(|nums: Vec<u64>| ids(&nums));
                assert_eq!(order, expected);
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
