use std::cmp::min;
use std::collections::{HashMap, HashSet};

use crate::dfs::{Dfs, Status, Visit};
use crate::{Graph, Partition};

struct NodeState {
    on_stack: bool,
    index: usize,
    lowlink: usize,
}

impl NodeState {
    fn new(index: usize) -> NodeState {
        NodeState {
            on_stack: true,
            index: index,
            lowlink: index,
        }
    }
}

pub(crate) struct Tarjan<'a, G: Graph + ?Sized> {
    g: &'a G,
    dfs: Dfs<'a, G>,
    stack: Vec<G::Node>,
    node_states: HashMap<G::Node, NodeState>,
    next_index: usize,
}

impl<'a, G: Graph + ?Sized> Tarjan<'a, G> {
    pub fn from_graph(g: &'a G) -> Self {
        Tarjan {
            g,
            dfs: g.dfs(),
            stack: Vec::new(),
            node_states: HashMap::new(),
            next_index: 0,
        }
    }

    pub fn run(mut self) -> Partition<G> {
        let mut ret = Vec::new();

        for visit in self.dfs {
            match visit {
                Visit::Retreat { u, parent } => {
                    let lowlink = self.node_states[&u].lowlink;
                    let index = self.node_states[&u].index;

                    if let Some(p) = parent {
                        self.node_states
                            .entry(p)
                            .and_modify(|s| s.lowlink = min(s.lowlink, lowlink));
                    }

                    if lowlink == index {
                        // u is the root of a strongly connected component, which consists of all
                        // the nodes that are above u in the stack.
                        let mut scc = HashSet::new();
                        loop {
                            // The unwrap is ok here: when we start the loop, u is guaranteed to be
                            // in the stack. Since we stop the loop whenever we find u, we're
                            // guaranteed never to run out of stack.
                            let v = self.stack.pop().unwrap();
                            self.node_states
                                .entry(v)
                                .and_modify(|s| s.on_stack = false);
                            scc.insert(v.clone());
                            if v == u {
                                break;
                            }
                        }
                        ret.push(scc);
                    }
                }
                Visit::Root(u) => {
                    self.stack.push(u.clone());
                    self.node_states.insert(u, NodeState::new(self.next_index));
                    self.next_index += 1;
                }
                Visit::Edge { src, dst, status } => {
                    if status == Status::New {
                        // The DFS is about to recurse on the destination node, so we'll update our
                        // state to reflect that.
                        self.stack.push(dst.clone());
                        self.node_states
                            .insert(dst, NodeState::new(self.next_index));
                        self.next_index += 1;
                    } else if self.node_states[&dst].on_stack {
                        // The fact that dst is on the stack implies that there is a path from dst
                        // to src.
                        let index = self.node_states[&dst].index;
                        self.node_states
                            .entry(src)
                            .and_modify(|s| s.lowlink = min(s.lowlink, index));
                    }
                }
            }
        }

        ret.reverse();
        Partition::new(self.g, ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{arb_dag, arb_graph, graph};
    use crate::Graph;

    macro_rules! tarjan_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let d = g.tarjan();
                let expected: Vec<_> = $expected
                    .into_iter()
                    .map(|scc| scc.into_iter().cloned().collect::<HashSet<u32>>())
                    .collect();
                assert_eq!(d.sets, expected);
            }
        };
    }

    tarjan_test!(triangle, "0-1, 1-2, 2-0", [[0, 1, 2]]);
    tarjan_test!(
        two_triangles,
        "0-1, 1-2, 2-0, 2-3, 3-4, 4-5, 5-3",
        [[0, 1, 2], [3, 4, 5]]
    );
    tarjan_test!(
        diamond,
        "0-1, 0-2, 1-3, 2-3",
        [[0], [2], [1], [3]]
    );

    proptest! {
        #[test]
        fn tarjan_dag_proptest(ref g in arb_dag()) {
            let sccs = g.tarjan();
            for s in sccs.parts() {
                assert_eq!(s.len(), 1);
            }
        }

        #[test]
        fn tarjan_graph_proptest(ref g in arb_graph()) {
            let sccs = g.tarjan();
            for s in sccs.parts() {
                for t in sccs.parts() {
                    if s == t && s.len() > 1 {
                        for u in s {
                            for v in s {
                                assert!(g.has_path(u, v));
                            }
                        }
                    } else if s != t {
                        for u in s {
                            for v in t {
                                assert!(!(g.has_path(u, v) && g.has_path(v, u)));
                            }
                        }
                    }
                }
            }
        }

        #[test]
        fn tarjan_scc_dag(ref g in arb_graph()) {
            let sccs = g.tarjan();
            assert!(sccs.top_sort().is_some());
        }
    }
}

