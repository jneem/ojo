use crate::patch::{Change, Changes};
use crate::{NodeId, PatchId};
use multimap::MMap;
use std::collections::{BTreeMap as Map, HashSet};

pub mod digle;
pub mod file;

pub use self::digle::{Digle, DigleMut};
pub use self::file::File;

use self::digle::DigleData;

/// A unique identifier for a [`Digle`] in this repository.
///
/// Since we currently only support a single Digle per branch, `INode`s are in one-to-one
/// correspondence with branches. However, branches may be renamed while `INode`s are immutable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct INode {
    n: u64,
}

// This contains all of the "large" data in the repository; that is, all the parts that grow as the
// repository history grows. A real implementation would need to page in this storage on-demand
// and would also need to implement copy-on-write in various important places. For now, though, we
// just serialize and deserialize as a giant chunk.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Storage {
    // We generate unique INodes by assigning numbers in an increasing sequence. This is the next
    // one to be assigned.
    next_inode: u64,

    // These are the actual, textual contents of the lines. If we wanted to be clever, we could do
    // deduplication and/or compression.
    contents: Map<NodeId, Vec<u8>>,

    // This is a map from the names of branches to the inodes where those branches' data is stored.
    branches: Map<String, INode>,

    // This is a map from inodes to the actual data contained in them.
    digles: Map<INode, DigleData>,

    // A list of all the patches that we know about, and have ever known about. The contents of the
    // patches are not stored here; they live in a different directory, with one patch per file.
    pub patches: HashSet<PatchId>,

    // If this contains the key-value pair (branch, patch), it means that the named branch contains
    // the named patch.
    pub branch_patches: MMap<String, PatchId>,

    // If this contains the key-value pair (p1, p2), it means that patch p1 depends on patch p2.
    // (The same information can be obtained by reading the file containing patch p1, but it's more
    // convenient to keep a copy here.)
    pub patch_deps: MMap<PatchId, PatchId>,

    // This is the reverse of `patch_deps`: if this contains the key-value pair (p1, p2), it means
    // that patch p2 depends on patch p1.
    pub patch_rev_deps: MMap<PatchId, PatchId>,
}

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

    pub fn clone_inode(&mut self, inode: INode) -> INode {
        let ret = INode { n: self.next_inode };
        self.next_inode += 1;

        let old_digle = self.digles.get(&inode).unwrap();
        self.digles.insert(ret, old_digle.clone());
        ret
    }

    pub fn contents(&self, id: &NodeId) -> &[u8] {
        self.contents.get(id).unwrap().as_slice()
    }

    /// Panics if the node already has contents that differ from the current ones.
    pub fn add_contents(&mut self, id: NodeId, contents: Vec<u8>) {
        use std::collections::btree_map::Entry;

        match self.contents.entry(id) {
            Entry::Occupied(o) => assert_eq!(o.get(), &contents, "contents mismatch"),
            Entry::Vacant(v) => {
                v.insert(contents);
            }
        }
    }

    pub fn remove_contents(&mut self, id: &NodeId) {
        self.contents.remove(id);
    }

    pub fn inode(&self, branch: &str) -> Option<INode> {
        self.branches.get(branch).cloned()
    }

    pub fn set_inode(&mut self, branch: &str, inode: INode) -> Option<INode> {
        self.branches.insert(branch.to_owned(), inode)
    }

    pub fn remove_inode(&mut self, branch: &str) {
        self.branches.remove(branch);
    }

    pub fn update_cache(&mut self, inode: INode) {
        let digle = self.digles.get_mut(&inode).unwrap();
        digle.resolve_pseudo_edges();
    }

    pub fn digle_mut<'a>(&'a mut self, inode: INode) -> DigleMut<'a> {
        self.digles.get_mut(&inode).unwrap().into()
    }

    pub fn digle<'a>(&'a self, inode: INode) -> Digle<'a> {
        self.digles.get(&inode).unwrap().into()
    }

    pub fn remove_digle(&mut self, inode: INode) {
        self.digles.remove(&inode);
    }

    pub fn set_digle(&mut self, inode: INode, digle: DigleData) {
        self.digles.insert(inode, digle);
    }

    pub fn branches(&self) -> impl Iterator<Item = &str> {
        self.branches.keys().map(|s| s.as_str())
    }

    pub fn apply_changes(&mut self, inode: INode, changes: &Changes) {
        let digle = self.digles.get_mut(&inode).unwrap();
        for ch in &changes.changes {
            match *ch {
                Change::NewNode { ref id, .. } => digle.add_node(id.clone()),
                Change::DeleteNode { ref id } => digle.delete_node(&id),
                Change::NewEdge { ref src, ref dst } => digle.add_edge(src.clone(), dst.clone()),
            }
        }

        // Because `entry` borrows self.digles, the borrow checker isn't smart enough to allow this
        // into the previous loop.
        for ch in &changes.changes {
            if let Change::NewNode {
                ref id,
                ref contents,
            } = *ch
            {
                self.add_contents(id.clone(), contents.to_owned());
            }
        }
    }

    pub fn unapply_changes(&mut self, inode: INode, changes: &Changes) {
        let digle = self.digles.get_mut(&inode).unwrap();

        // Because of the requirements of `unadd_edge`, we need to unadd all edges before we unadd
        // all nodes.
        for ch in &changes.changes {
            match *ch {
                Change::DeleteNode { ref id } => digle.undelete_node(id),
                Change::NewEdge { ref src, ref dst } => digle.unadd_edge(src, dst),
                Change::NewNode { .. } => {}
            }
        }
        for ch in &changes.changes {
            if let Change::NewNode { ref id, .. } = *ch {
                digle.unadd_node(id);
            }
        }

        // Because `entry` borrows self.digles, the borrow checker isn't smart enough to allow this
        // into the previous loop.
        for ch in &changes.changes {
            if let Change::NewNode { ref id, .. } = *ch {
                self.remove_contents(id);
            }
        }
    }
}
