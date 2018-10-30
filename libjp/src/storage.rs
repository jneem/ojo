use crate::{LineId, PatchId};
use multimap::MMap;
use std::collections::{BTreeMap as Map, BTreeSet as Set, HashSet};

pub mod file;

pub use self::file::File;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct INode {
    n: u64,
}

/// This struct represents a directed edge in a digle graph.
///
/// Note that we don't actually store the source line, only the destination. However, the main way
/// of getting access to an `Edge` is via the `Digle::out_edges` or `Digle::in_edges` functions, so
/// usually you will only encounter an `Edge` if you already know what the source line is.
///
/// Note that edges are ordered, and that live edges will always come before deleted edges. This
/// helps ensure quick access to live edges.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    /// This will be `true` if it points to a line that was deleted.
    pub deleted: bool,
    /// The destination of this (directed) edge.
    pub dest: LineId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "Digle")]
struct DigleData {
    lines: Set<LineId>,
    deleted_lines: Set<LineId>,
    edges: MMap<LineId, Edge>,
    back_edges: MMap<LineId, Edge>,
}

impl DigleData {
    fn new() -> DigleData {
        DigleData {
            lines: Set::new(),
            deleted_lines: Set::new(),
            edges: MMap::new(),
            back_edges: MMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Digle<'a> {
    data: &'a DigleData,
}

impl<'a> Digle<'a> {
    pub fn out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b
    {
        self.data.edges.get(line).take_while(|e| !e.deleted)
    }

    pub fn all_out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b
    {
        self.data.edges.get(line)
    }

    pub fn in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b
    {
        self.data.back_edges.get(line).take_while(|e| !e.deleted)
    }

    pub fn all_in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b
    {
        self.data.back_edges.get(line)
    }

    pub fn is_live(&self, line: &LineId) -> bool {
        assert!(self.data.lines.contains(line) || self.data.deleted_lines.contains(line));
        self.data.lines.contains(line)
    }

    pub fn assert_consistent(&self) {
        // The live and deleted lines should be disjoint.
        assert!(self.data.lines.is_disjoint(&self.data.deleted_lines));

        let line_exists = |line_id|
            self.data.lines.contains(line_id) || self.data.deleted_lines.contains(line_id);
        // The source and destination of every edge should exist somewhere.
        // The `deleted` field of an edge should agree with the status of the destination line.
        // There should be a one-to-one correspondence between edges and back_edges.
        for (line, edge) in self.data.edges.iter() {
            assert!(line_exists(line));
            assert!(line_exists(&edge.dest));
            assert_eq!(edge.deleted, self.data.deleted_lines.contains(&edge.dest));
            let back_edge = Edge {
                dest: line.clone(),
                deleted: self.data.deleted_lines.contains(line),
            };
            assert!(self.data.back_edges.contains(&edge.dest, &back_edge));
        }
        for (line, back_edge) in self.data.back_edges.iter() {
            assert!(line_exists(line));
            assert!(line_exists(&back_edge.dest));
            assert_eq!(back_edge.deleted, self.data.deleted_lines.contains(&back_edge.dest));
            let edge = Edge {
                dest: line.clone(),
                deleted: self.data.deleted_lines.contains(line),
            };
            assert!(self.data.edges.contains(&back_edge.dest, &edge));
        }
    }
}

#[derive(Debug)]
pub struct DigleMut<'a> {
    data: &'a mut DigleData,
}

impl<'a> DigleMut<'a> {
    pub fn as_digle<'b>(&'b self) -> Digle<'b> {
        Digle { data: self.data }
    }

    pub fn add_node(&mut self, id: LineId) {
        self.data.lines.insert(id);
    }

    pub fn unadd_node(&mut self, id: &LineId) {
        // If we are unadding a line, it means we are unapplying the patch in which the line was
        // introduced. Since we must have already unapplied any reverse-dependencies of the patch,
        // the line must be live (it can't have been marked as deleted).
        assert!(self.data.lines.contains(id));
        self.data.lines.remove(id);
    }

    pub fn delete_node(&mut self, id: &LineId) {
        assert!(self.data.lines.contains(id));
        self.data.lines.remove(id);
        self.data.deleted_lines.insert(id.clone());

        // All the edges (both forward and backwards) pointing towards the newly deleted node need
        // to be marked as deleted.
        let out_neighbors = self.as_digle()
            .all_out_edges(id)
            .map(|e| e.dest.clone())
            .collect::<Vec<_>>();
        let in_neighbors = self.as_digle()
            .all_in_edges(id)
            .map(|e| e.dest.clone())
            .collect::<Vec<_>>();
        for o in out_neighbors {
            self.mark_back_edge(o, id.clone(), true);
        }
        for i in in_neighbors {
            self.mark_edge(i, id.clone(), true);
        }
    }

    pub fn undelete_node(&mut self, id: &LineId) {
        assert!(self.data.deleted_lines.contains(id));
        self.data.deleted_lines.remove(id);
        self.data.lines.insert(id.clone());

        // All the edges (both forward and backwards) pointing towards the newly deleted node need
        // to be marked as live.
        let out_neighbors = self.as_digle()
            .all_out_edges(id)
            .map(|e| e.dest.clone())
            .collect::<Vec<_>>();
        let in_neighbors = self.as_digle()
            .all_in_edges(id)
            .map(|e| e.dest.clone())
            .collect::<Vec<_>>();
        for o in out_neighbors {
            self.mark_back_edge(o, id.clone(), false);
        }
        for i in in_neighbors {
            self.mark_edge(i, id.clone(), false);
        }
    }

    // If `delete` is true, marks a back_edge as deleted. Otherwise, marks it as undeleted.
    fn mark_back_edge(&mut self, src: LineId, dst: LineId, delete: bool) {
        // Note that because changing the deletion flag affects the order in the map, we actually
        // have to delete the edge first, then modify it, then re-insert.
        let mut e = Edge {
            deleted: !delete,
            dest: dst.clone(),
        };
        self.data.back_edges.remove(&src, &e);
        e.deleted = delete;
        self.data.back_edges.insert(src, e);
    }

    // If `delete` is true, marks an edge as deleted. Otherwise, marks it as undeleted.
    fn mark_edge(&mut self, src: LineId, dst: LineId, delete: bool) {
        let mut e = Edge {
            deleted: !delete,
            dest: dst.clone(),
        };
        self.data.edges.remove(&src, &e);
        e.deleted = delete;
        self.data.edges.insert(src, e);
    }

    pub fn add_edge(&mut self, from: LineId, to: LineId) {
        let from_deleted = !self.data.lines.contains(&from);
        let to_deleted = !self.data.lines.contains(&to);
        assert!(!from_deleted || self.data.deleted_lines.contains(&from));
        assert!(!to_deleted || self.data.deleted_lines.contains(&to));

        self.data.edges.insert(
            from.clone(),
            Edge {
                deleted: to_deleted,
                dest: to.clone(),
            },
        );
        self.data.back_edges.insert(
            to,
            Edge {
                deleted: from_deleted,
                dest: from,
            },
        );
    }

    /// # Panics
    ///
    /// Panics unless `from` and `to` are lines in this digle. In particular, if you're planning to
    /// remove some lines and the edge between them, you need to remove the lines first.
    pub fn unadd_edge(&mut self, from: &LineId, to: &LineId) {
        let from_deleted = !self.data.lines.contains(&from);
        let to_deleted = !self.data.lines.contains(&to);
        assert!(!from_deleted || self.data.deleted_lines.contains(&from));
        assert!(!to_deleted || self.data.deleted_lines.contains(&to));

        let forward_edge = Edge {
            deleted: to_deleted,
            dest: to.clone(),
        };
        let back_edge = Edge {
            deleted: from_deleted,
            dest: from.clone(),
        };
        self.data.edges.remove(&from, &forward_edge);
        self.data.back_edges.remove(&to, &back_edge);
    }
}

impl<'a, 'b: 'a> crate::graph::GraphRef<'a> for &'a Digle<'b> {
    // TODO: once impl Trait return types are nameable, unbox these
    type NodesIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;
    type OutNeighborsIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;
    type InNeighborsIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;

    fn nodes(self) -> Self::NodesIter {
        Box::new(self.data.lines.iter().chain(self.data.deleted_lines.iter()))
    }

    fn out_neighbors(self, u: &LineId) -> Self::OutNeighborsIter {
        Box::new(self.all_out_edges(u).map(|e| &e.dest))
    }

    fn in_neighbors(self, u: &LineId) -> Self::InNeighborsIter {
        Box::new(self.all_in_edges(u).map(|e| &e.dest))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Storage {
    next_inode: u64,
    contents: Map<LineId, Vec<u8>>,
    branches: Map<String, INode>,
    digles: Map<INode, DigleData>,

    pub(crate) patches: HashSet<PatchId>,
    // If this contains the key-value pair (branch, patch), it means that the named branch contains
    // the named patch.
    pub(crate) branch_patches: MMap<String, PatchId>,
    pub(crate) patch_deps: MMap<PatchId, PatchId>,
    pub(crate) patch_rev_deps: MMap<PatchId, PatchId>,
}

// Everything in storage should be copy-on-write. That is, I should be able to get a read-only
// copy, then I should be able to get a writable copy from that. I should store the writable copy
// back in the storage.
impl Storage {
    pub fn new() -> Storage {
        Storage {
            next_inode: 0,
            contents: Map::new(),
            branches: Map::new(),
            digles: Map::new(),
            patches: HashSet::new(),
            branch_patches: MMap::new(),
            patch_deps: MMap::new(),
            patch_rev_deps: MMap::new(),
        }
    }

    pub fn allocate_inode(&mut self) -> INode {
        let ret = INode { n: self.next_inode };
        self.next_inode += 1;

        self.digles.insert(ret, DigleData::new());
        ret
    }

    pub fn contents(&self, id: &LineId) -> &[u8] {
        self.contents.get(id).unwrap().as_slice()
    }

    /// Panics if the line already has contents.
    pub fn add_contents(&mut self, id: LineId, contents: Vec<u8>) {
        assert!(!self.contents.contains_key(&id));
        self.contents.insert(id, contents);
    }

    pub fn remove_contents(&mut self, id: &LineId) {
        self.contents.remove(id);
    }

    pub fn inode(&self, branch: &str) -> Option<INode> {
        self.branches.get(branch).cloned()
    }

    pub fn set_inode(&mut self, branch: &str, inode: INode) -> Option<INode> {
        self.branches.insert(branch.to_owned(), inode)
    }

    pub fn digle_mut<'a>(&'a mut self, inode: INode) -> DigleMut<'a> {
        DigleMut {
            data: self.digles.get_mut(&inode).unwrap(),
        }
    }

    pub fn digle<'a>(&'a self, inode: INode) -> Digle<'a> {
        Digle {
            data: self.digles.get(&inode).unwrap(),
        }
    }

    pub fn branches(&self) -> impl Iterator<Item=&str> {
        self.branches.keys().map(|s| s.as_str())
    }
}

