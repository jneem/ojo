// Copyright 2018-2019 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// See the LICENSE-APACHE or LICENSE-MIT files at the top-level directory
// of this distribution.

use crate::patch::{Change, Changes};
use crate::{NodeId, PatchId};
use ojo_multimap::MMap;
use std::collections::{BTreeMap, HashMap};

#[macro_use]
pub mod graggle;
pub mod file;

pub use self::file::File;
pub use self::graggle::{FullGraph, Graggle, LiveGraph};

use self::graggle::GraggleData;

/// A unique identifier for a [`Graggle`] in this repository.
///
/// Since we currently only support a single Graggle per branch, `INode`s are in one-to-one
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
    contents: BTreeMap<NodeId, Vec<u8>>,

    // This is a map from the names of branches to the inodes where those branches' data is stored.
    branches: BTreeMap<String, INode>,

    // This is a map from inodes to the actual data contained in them.
    graggles: BTreeMap<INode, GraggleData>,

    // These are all the patches that we know about, and have ever known about.
    //
    // The contents of the patches are YAML.
    pub patches: HashMap<PatchId, String>,

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
            contents: BTreeMap::new(),
            branches: BTreeMap::new(),
            graggles: BTreeMap::new(),
            patches: HashMap::new(),
            branch_patches: MMap::new(),
            patch_deps: MMap::new(),
            patch_rev_deps: MMap::new(),
        }
    }

    pub fn allocate_inode(&mut self) -> INode {
        let ret = INode { n: self.next_inode };
        self.next_inode += 1;

        self.graggles.insert(ret, GraggleData::new());
        ret
    }

    pub fn clone_inode(&mut self, inode: INode) -> INode {
        let ret = INode { n: self.next_inode };
        self.next_inode += 1;

        let old_graggle = &self.graggles[&inode];
        self.graggles.insert(ret, old_graggle.clone());
        ret
    }

    pub fn contents(&self, id: &NodeId) -> &[u8] {
        self.contents[id].as_slice()
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

    pub fn contains_node(&self, id: &NodeId) -> bool {
        self.contents.contains_key(id)
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
        let graggle = self.graggles.get_mut(&inode).unwrap();
        graggle.resolve_pseudo_edges();
    }

    pub fn graggle(&'_ self, inode: INode) -> Graggle<'_> {
        self.graggles[&inode].as_graggle()
    }

    pub fn remove_graggle(&mut self, inode: INode) {
        self.graggles.remove(&inode);
    }

    pub fn set_graggle(&mut self, inode: INode, graggle: GraggleData) {
        self.graggles.insert(inode, graggle);
    }

    pub fn branches(&self) -> impl Iterator<Item = &str> {
        self.branches.keys().map(|s| s.as_str())
    }

    pub fn apply_changes(&mut self, inode: INode, changes: &Changes, patch: PatchId) {
        let graggle = self.graggles.get_mut(&inode).unwrap();
        for ch in &changes.changes {
            match *ch {
                Change::NewNode { ref id, .. } => {
                    debug!("adding node {:?}", id);
                    graggle.add_node(id.clone());
                }
                Change::DeleteNode { ref id } => {
                    debug!("deleting node {:?}", id);
                    graggle.delete_node(&id);
                }
                Change::NewEdge { ref src, ref dest } => {
                    debug!("adding edge {:?} -- {:?}", src, dest);
                    graggle.add_edge(src.clone(), dest.clone(), patch);
                }
            }
        }

        // Because we borrowed self.graggles, the borrow checker isn't smart enough to allow this
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

    pub fn unapply_changes(&mut self, inode: INode, changes: &Changes, patch: PatchId) {
        let graggle = self.graggles.get_mut(&inode).unwrap();

        // Because of the requirements of `unadd_edge`, we need to unadd all edges before we unadd
        // all nodes.
        for ch in &changes.changes {
            match *ch {
                Change::DeleteNode { ref id } => {
                    debug!("undeleting node {:?}", id);
                    graggle.undelete_node(id);
                }
                Change::NewEdge { ref src, ref dest } => {
                    debug!("unadding edge {:?} -- {:?}", src, dest);
                    graggle.unadd_edge(src, dest, patch);
                }
                Change::NewNode { .. } => {}
            }
        }
        for ch in &changes.changes {
            if let Change::NewNode { ref id, .. } = *ch {
                debug!("unadding node {:?}", id);
                graggle.unadd_node(id);
            }
        }

        // Because we borrowed self.graggles, the borrow checker isn't smart enough to allow this
        // into the previous loop.
        for ch in &changes.changes {
            if let Change::NewNode { ref id, .. } = *ch {
                self.remove_contents(id);
            }
        }
    }
}
