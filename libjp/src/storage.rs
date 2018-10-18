use multimap::MMap;
use rpds::{RedBlackTreeMap as Map, RedBlackTreeSet as Set};
use crate::{Edge, LineId};

pub mod file;

pub use self::file::File;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct INode {
    n: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Digle {
    lines: Set<LineId>,
    edges: MMap<LineId, Edge>,
    back_edges: MMap<LineId, Edge>,
}

impl Digle {
    pub fn new() -> Digle {
        Digle {
            lines: Set::new(),
            edges: MMap::new(),
            back_edges: MMap::new(),
        }
    }

    pub fn out_edges<'a>(&'a self, line: LineId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.edges.get(&line)
    }

    pub fn in_edges<'a>(&'a self, line: LineId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.back_edges.get(&line)
    }

    pub fn add_node(&mut self, id: LineId) {
        self.lines.insert_mut(id);
    }

    pub fn delete_node(&mut self, id: &LineId) {
        unimplemented!();
    }

    pub fn add_edge(&mut self, from: LineId, to: LineId) {
        assert!(self.lines.contains(&from));
        assert!(self.lines.contains(&to));

        self.edges.insert_mut(from.clone(), Edge { dest: to.clone() });
        self.back_edges.insert_mut(to, Edge { dest: from });
    }
}

impl<'a> crate::graph::GraphRef<'a> for &'a Digle {
    // TODO: once impl Trait return types are nameable, unbox these
    type NodesIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;
    type OutNeighborsIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;
    type InNeighborsIter = Box<dyn Iterator<Item = &'a LineId> + 'a>;

    fn nodes(self) -> Self::NodesIter {
        Box::new(self.lines.iter())
    }

    fn out_neighbors(self, u: &LineId) -> Self::OutNeighborsIter {
        Box::new(self.out_edges(u.clone()).map(|e| &e.dest))
    }

    fn in_neighbors(self, u: &LineId) -> Self::InNeighborsIter {
        Box::new(self.in_edges(u.clone()).map(|e| &e.dest))
    }
}

// Maybe it's overkill to use persistent maps for contents and branches. For sure, we want them for
// the digles because we need digles in different branches to share data.
#[derive(Debug, Deserialize, Serialize)]
pub struct Storage {
    contents: Map<LineId, Vec<u8>>,
    branches: Map<String, INode>,
    digles: Map<INode, Digle>,
}

// Everything in storage should be copy-on-write. That is, I should be able to get a read-only
// copy, then I should be able to get a writable copy from that. I should store the writable copy
// back in the storage.
impl Storage {
    pub fn new() -> Storage {
        Storage {
            contents: Map::new(),
            branches: Map::new(),
            digles: Map::new(),
        }
    }

    pub fn allocate_inode(&mut self) -> INode {
        //FIXME
        let ret = INode { n: 0 };
        let digle = Digle::new();
        self.digles = self.digles.insert(ret, digle);
        ret
    }

    pub fn contents(&self, id: &LineId) -> &[u8] {
        self.contents.get(id).unwrap().as_slice()
    }

    /// Panics if the line already has contents.
    pub fn add_contents(&mut self, id: LineId, contents: Vec<u8>) {
        assert!(!self.contents.contains_key(&id));
        self.contents = self.contents.insert(id, contents);
    }

    pub fn inode(&self, branch: &str) -> Option<INode> {
        self.branches.get(branch).cloned()
    }

    pub fn set_inode(&mut self, branch: &str, inode: INode) -> Option<INode> {
        let ret = self.inode(branch);
        self.branches = self.branches.insert(branch.to_owned(), inode);
        ret
    }

    pub fn digle(&self, inode: INode) -> Digle {
        self.digles.get(&inode).unwrap().clone()
    }
}
