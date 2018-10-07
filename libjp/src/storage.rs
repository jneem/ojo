use multimap::MMap;
use rpds::{RedBlackTreeMap as Map, RedBlackTreeSet as Set};
use {Edge, LineId};

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

    pub fn out_edges<'a>(&'a self, line: LineId) -> impl Iterator<Item = Edge> + 'a {
        self.edges.get(&line).cloned()
    }

    pub fn in_edges<'a>(&'a self, line: LineId) -> impl Iterator<Item = Edge> + 'a {
        self.back_edges.get(&line).cloned()
    }

    pub fn add_node(&self, id: LineId) -> Digle {
        Digle {
            lines: self.lines.insert(id),
            edges: self.edges.clone(),
            back_edges: self.back_edges.clone(),
        }
    }

    pub fn add_edge(&self, from: LineId, to: LineId) -> Digle {
        assert!(self.lines.contains(&from));
        assert!(self.lines.contains(&to));

        let new_edges = self.edges.insert(from.clone(), Edge { dest: to.clone() });
        let new_back_edges = self.back_edges.insert(to, Edge { dest: from });
        Digle {
            lines: self.lines.clone(),
            edges: new_edges,
            back_edges: new_back_edges,
        }
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

    pub fn contents(&self, id: LineId) -> &[u8] {
        self.contents.get(&id).unwrap().as_slice()
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
