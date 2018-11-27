use std::collections::HashSet;

use crate::{Edge, Graph};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    New,
    Repeated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Visit<N> {
    Edge { src: N, dst: N, status: Status },
    Retreat { u: N, parent: Option<N> },
    Root(N),
}

// A naive DFS is recursive, and so it can run out of stack space. Here, we prevent that by
// manually maintaining our own stack. Each frame needs to keep track of the node being explored
// and which of its neighbors remain to explore.
//
// (There is also a simpler non-recursive way to write DFS (described, e.g. on wikipedia), but that
// one loses information about which edges we're traversing.)
struct StackFrame<'a, G: Graph + ?Sized> {
    u: G::Node,
    neighbors: Box<dyn Iterator<Item = G::Edge> + 'a>,
}

impl<'a, G: Graph + ?Sized> StackFrame<'a, G> {
    fn new(g: &'a G, u: G::Node) -> StackFrame<'a, G> {
        StackFrame {
            neighbors: g.out_edges(&u),
            u: u,
        }
    }
}

pub struct Dfs<'a, G: Graph + ?Sized> {
    g: &'a G,
    visited: HashSet<G::Node>,
    stack: Vec<StackFrame<'a, G>>,
    roots: Box<dyn Iterator<Item = G::Node> + 'a>,
}

impl<'a, G: Graph + ?Sized> Dfs<'a, G> {
    pub(crate) fn new(g: &'a G) -> Dfs<'a, G> {
        Dfs {
            g: g,
            visited: HashSet::new(),
            stack: Vec::new(),
            roots: g.nodes(),
        }
    }

    pub(crate) fn new_from(g: &'a G, root: &G::Node) -> Dfs<'a, G> {
        Dfs {
            g: g,
            visited: HashSet::new(),
            stack: Vec::new(),
            roots: Box::new(Some(*root).into_iter()),
        }
    }

    fn next_root(&mut self) -> Option<G::Node> {
        while let Some(root) = self.roots.next() {
            if !self.visited.contains(&root) {
                return Some(root);
            }
        }
        None
    }

    fn cur_node(&self) -> Option<G::Node> {
        self.stack.last().map(|frame| frame.u)
    }
}

impl<'a, G: Graph + ?Sized> Iterator for Dfs<'a, G> {
    type Item = Visit<G::Node>;

    fn next(&mut self) -> Option<Visit<G::Node>> {
        if let Some(frame) = self.stack.last_mut() {
            let cur = frame.u;
            if let Some(next) = frame.neighbors.next() {
                let next = next.target();
                let status = if self.visited.contains(&next) {
                    Status::Repeated
                } else {
                    self.stack.push(StackFrame::new(self.g, next));
                    self.visited.insert(next);
                    Status::New
                };
                Some(Visit::Edge {
                    src: cur,
                    dst: next,
                    status: status,
                })
            } else {
                self.stack.pop();
                Some(Visit::Retreat {
                    u: cur,
                    parent: self.cur_node(),
                })
            }
        } else if let Some(next_root) = self.next_root() {
            self.stack.push(StackFrame::new(self.g, next_root));
            self.visited.insert(next_root);
            Some(Visit::Root(next_root))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Status::*;
    use super::Visit::*;
    use crate::tests::graph;
    use crate::Graph;

    macro_rules! dfs_test {
        ($name:ident, $graph:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let g = graph($graph);
                let dfs: Vec<_> = g.dfs().collect();
                assert_eq!(dfs, $expected);
            }
        };
    }

    dfs_test!(
        visit_order,
        "0-1, 0-3, 0-2",
        vec![
            Root(0),
            Edge {
                src: 0,
                dst: 1,
                status: New
            },
            Retreat {
                u: 1,
                parent: Some(0)
            },
            Edge {
                src: 0,
                dst: 3,
                status: New
            },
            Retreat {
                u: 3,
                parent: Some(0)
            },
            Edge {
                src: 0,
                dst: 2,
                status: New
            },
            Retreat {
                u: 2,
                parent: Some(0)
            },
            Retreat { u: 0, parent: None },
        ]
    );

    dfs_test!(
        repeat_visit,
        "0-1, 0-2, 1-2",
        vec![
            Root(0),
            Edge {
                src: 0,
                dst: 1,
                status: New
            },
            Edge {
                src: 1,
                dst: 2,
                status: New
            },
            Retreat {
                u: 2,
                parent: Some(1)
            },
            Retreat {
                u: 1,
                parent: Some(0)
            },
            Edge {
                src: 0,
                dst: 2,
                status: Repeated
            },
            Retreat { u: 0, parent: None },
        ]
    );
}
