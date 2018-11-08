use std::collections::HashSet;

use crate::graph::GraphRef;
use crate::LineId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    New,
    Repeated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Visit {
    Edge {
        src: LineId,
        dst: LineId,
        status: Status,
    },
    Retreat {
        u: LineId,
        parent: Option<LineId>,
    },
    Root(LineId),
}

// A naive DFS is recursive, and so it can run out of stack space. Here, we prevent that by
// manually maintaining our own stack. Each frame needs to keep track of the node being explored
// and which of its neighbors remain to explore.
//
// (There is also a simpler non-recursive way to write DFS (described, e.g. on wikipedia), but that
// one loses information about which edges we're traversing.)
struct StackFrame<'a, G: GraphRef<'a> + ?Sized> {
    u: LineId,
    neighbors: G::OutNeighborsIter,
}

impl<'a, G: GraphRef<'a> + ?Sized> StackFrame<'a, G> {
    fn new(g: G, u: LineId) -> StackFrame<'a, G> {
        StackFrame {
            neighbors: g.out_neighbors(&u),
            u: u,
        }
    }
}

pub struct Dfs<'a, G: GraphRef<'a> + ?Sized> {
    g: G,
    visited: HashSet<LineId>,
    stack: Vec<StackFrame<'a, G>>,
    roots: G::NodesIter,
}

impl<'a, G: GraphRef<'a> + ?Sized + 'a> Dfs<'a, G> {
    pub(crate) fn new(g: G) -> Dfs<'a, G> {
        Dfs {
            g: g,
            visited: HashSet::new(),
            stack: Vec::new(),
            roots: g.nodes(),
        }
    }

    fn next_root(&mut self) -> Option<LineId> {
        while let Some(root) = self.roots.next() {
            if !self.visited.contains(root) {
                return Some(root.clone());
            }
        }
        None
    }

    fn cur_node(&self) -> Option<LineId> {
        self.stack.last().map(|frame| frame.u.clone())
    }
}

impl<'a, G: GraphRef<'a> + ?Sized> Iterator for Dfs<'a, G> {
    type Item = Visit;

    fn next(&mut self) -> Option<Visit> {
        if let Some(frame) = self.stack.last_mut() {
            let cur = frame.u.clone();
            if let Some(next) = frame.neighbors.next() {
                let status = if self.visited.contains(&next) {
                    Status::Repeated
                } else {
                    self.stack.push(StackFrame::new(self.g, next.clone()));
                    self.visited.insert(next.clone());
                    Status::New
                };
                Some(Visit::Edge {
                    src: cur,
                    dst: next.clone(),
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
            self.stack.push(StackFrame::new(self.g, next_root.clone()));
            self.visited.insert(next_root.clone());
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
    use crate::graph::tests::{graph, id};
    use crate::graph::GraphRef;

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
            Root(id(0)),
            Edge {
                src: id(0),
                dst: id(1),
                status: New
            },
            Retreat {
                u: id(1),
                parent: Some(id(0))
            },
            Edge {
                src: id(0),
                dst: id(3),
                status: New
            },
            Retreat {
                u: id(3),
                parent: Some(id(0))
            },
            Edge {
                src: id(0),
                dst: id(2),
                status: New
            },
            Retreat {
                u: id(2),
                parent: Some(id(0))
            },
            Retreat {
                u: id(0),
                parent: None
            },
        ]
    );

    dfs_test!(
        repeat_visit,
        "0-1, 0-2, 1-2",
        vec![
            Root(id(0)),
            Edge {
                src: id(0),
                dst: id(1),
                status: New
            },
            Edge {
                src: id(1),
                dst: id(2),
                status: New
            },
            Retreat {
                u: id(2),
                parent: Some(id(1))
            },
            Retreat {
                u: id(1),
                parent: Some(id(0))
            },
            Edge {
                src: id(0),
                dst: id(2),
                status: Repeated
            },
            Retreat {
                u: id(0),
                parent: None
            },
        ]
    );
}
