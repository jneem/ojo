use crate::{LineId, PatchId};
use crate::patch::{Change, Changes};
use multimap::MMap;
use std::collections::{BTreeMap as Map, HashSet};

pub mod cached_digle;
pub mod digle;
pub mod file;

pub use self::digle::{Digle, DigleMut};
pub use self::file::File;

use self::cached_digle::CachedDigle;
use self::digle::DigleData;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct INode {
    n: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DigleEntry {
    original: DigleData,
    cached: CachedDigle,
}

impl DigleEntry {
    fn new() -> DigleEntry {
        DigleEntry {
            original: DigleData::new(),
            cached: CachedDigle::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Storage {
    next_inode: u64,
    contents: Map<LineId, Vec<u8>>,
    branches: Map<String, INode>,
    digles: Map<INode, DigleEntry>,

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

        self.digles.insert(ret, DigleEntry::new());
        ret
    }

    pub fn clone_inode(&mut self, inode: INode) -> INode {
        let ret = INode { n: self.next_inode };
        self.next_inode += 1;

        let old_digle = self.digles.get(&inode).unwrap();
        self.digles.insert(ret, old_digle.clone());
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

    pub fn remove_inode(&mut self, branch: &str) {
        self.branches.remove(branch);
    }

    pub fn update_cache(&mut self, inode: INode) {
        let entry = self.digles.get_mut(&inode).unwrap();
        entry.cached.resolve((&entry.original).into());
    }

    // TODO: should we really be giving access to the original digles (as opposed to the cached
    // ones)?
    pub fn digle_mut<'a>(&'a mut self, inode: INode) -> DigleMut<'a> {
        (&mut self.digles.get_mut(&inode).unwrap().original).into()
    }

    pub fn digle<'a>(&'a self, inode: INode) -> Digle<'a> {
        (&self.digles.get(&inode).unwrap().original).into()
    }

    pub fn remove_digle(&mut self, inode: INode) {
        self.digles.remove(&inode);
    }

    pub fn branches(&self) -> impl Iterator<Item = &str> {
        self.branches.keys().map(|s| s.as_str())
    }

    pub fn apply_changes(&mut self, inode: INode, changes: &Changes) {
        let entry = self.digles.get_mut(&inode).unwrap();
        entry.cached.apply_changes(&changes.changes);
        for ch in &changes.changes {
            match *ch {
                Change::NewNode { ref id, .. } => entry.original.add_node(id.clone()),
                Change::DeleteNode { ref id } => entry.original.delete_node(&id),
                Change::NewEdge { ref src, ref dst } => entry.original.add_edge(src.clone(), dst.clone()),
            }
        }

        // Because `entry` borrows self.digles, the borrow checker isn't smart enough to allow this
        // into the previous loop.
        for ch in &changes.changes {
            if let Change::NewNode { ref id, ref contents } = *ch {
                self.add_contents(id.clone(), contents.to_owned());
            }
        }
    }

    pub fn unapply_changes(&mut self, inode: INode, changes: &Changes) {
        let entry = self.digles.get_mut(&inode).unwrap();
        entry.cached.unapply_changes(&changes.changes);

        // Because of the requirements of `unadd_edge`, we need to unadd all edges before we unadd
        // all nodes.
        for ch in &changes.changes {
            match *ch {
                Change::DeleteNode { ref id } => entry.original.undelete_node(id),
                Change::NewEdge { ref src, ref dst } => entry.original.unadd_edge(src, dst),
                Change::NewNode { .. } => {},
            }
        }
        for ch in &changes.changes {
            if let Change::NewNode { ref id, .. } = *ch {
                entry.original.unadd_node(id);
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
