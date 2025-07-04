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

#![deny(missing_docs)]

//! A library for creating, reading, and manipulating `ojo` repositories.
//!
//! `ojo` is a toy implementation of a version control system inspired by the same ideas as
//! [`pijul`](https://pijul.com). These ideas, and eventually the implementation of `ojo`,
//! are documented in some [`blog posts`](https://jneem.github.io). This crate itself is not so
//! well documented, but doing so is one of my goals.

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use {
    ojo_graph::Graph,
    std::{
        collections::HashSet,
        fs,
        path::{Path, PathBuf},
    },
};

mod chain_graggle;
mod error;
mod patch;
pub mod resolver;
mod storage;

pub use {
    crate::{
        chain_graggle::ChainGraggle,
        error::{Error, PatchIdError},
        patch::{Change, Changes, Patch, PatchId, UnidentifiedPatch},
        storage::{
            File, FullGraph, Graggle, LiveGraph,
            graggle::{Edge, EdgeKind},
        },
    },
    ojo_diff::LineDiff,
};

/// A globally unique ID for identifying a node.
#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NodeId {
    /// The ID of the patch that first introduced this node.
    pub patch: PatchId,
    /// The index of this node within the patch.
    ///
    /// If a patch introduces `n` nodes, they are given `node` values of `0` through `n-1`.
    pub node: u64,
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_tuple("NodeId")
            .field(&format!("{}/{:?}", self.patch.to_base64(), self.node))
            .finish()
    }
}

impl NodeId {
    fn set_patch_id(&mut self, id: &PatchId) {
        if self.patch.is_cur() {
            self.patch = *id;
        }
    }

    /// Creates a new `NodeId` for referring to a node that is being introduced in the current
    /// patch.
    ///
    /// See [`PatchId`] for more information on the current patch and its ID.
    pub fn cur(node: u64) -> NodeId {
        NodeId {
            patch: PatchId::cur(),
            node,
        }
    }
}

/// This is the main interface to a `ojo` repository.
///
/// Be aware that any modifications made to a repository will not be saved unless [`Repo::write`]
/// is called.
#[derive(Debug)]
pub struct Repo {
    /// The path to the root directory of the repository.
    pub root_dir: PathBuf,
    /// The path to the directory where all of ojo's data is stored.
    pub repo_dir: PathBuf,
    /// The path to the database containing all the history, and so on.
    pub db_path: PathBuf,
    /// The path to the directory where patches are stored.
    /// The name of the current branch.
    pub current_branch: String,

    storage: storage::Storage,
}

impl Repo {
    /// Given the path of the root directory of a repository, returns the directory where ojo's data
    /// is stored.
    fn repo_dir(dir: &Path) -> Result<PathBuf, Error> {
        let mut ret = dir.to_path_buf();
        ret.push(".ojo");
        Ok(ret)
    }

    /// Given the path of the root directory of a repository, returns the path containing ojo's
    /// serialized data.
    fn db_path(dir: &Path) -> Result<PathBuf, Error> {
        let mut ret = Repo::repo_dir(dir)?;
        ret.push("db");
        Ok(ret)
    }

    /// Opens the existing repository with the given root directory.
    pub fn open<P: AsRef<Path>>(dir: P) -> Result<Repo, Error> {
        let db_path = Repo::db_path(dir.as_ref())?;
        let db_file = fs::File::open(&db_path)?;
        let db: Db = serde_yaml::from_reader(db_file)?;
        Ok(Repo {
            root_dir: dir.as_ref().to_owned(),
            repo_dir: Repo::repo_dir(dir.as_ref())?,
            db_path,
            current_branch: db.current_branch,
            storage: db.storage,
        })
    }

    /// Creates a repo at the given path (which should point to a directory).
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Repo, Error> {
        let root_dir = path.as_ref().to_owned();
        let repo_dir = Repo::repo_dir(&root_dir)?;
        let db_path = Repo::db_path(&root_dir)?;
        if db_path.exists() {
            return Err(Error::RepoExists(repo_dir.clone()));
        }

        let mut storage = storage::Storage::new();
        let master_inode = storage.allocate_inode();
        storage.set_inode("master", master_inode);
        Ok(Repo {
            root_dir,
            repo_dir,
            db_path,
            current_branch: "master".to_owned(),
            storage,
        })
    }

    /// Creates a temporary in-memory repo that cannot be stored.
    pub fn init_tmp() -> Repo {
        let mut storage = storage::Storage::new();
        let master_inode = storage.allocate_inode();
        storage.set_inode("master", master_inode);

        Repo {
            root_dir: PathBuf::new(),
            repo_dir: PathBuf::new(),
            db_path: PathBuf::new(),
            current_branch: "master".to_owned(),
            storage,
        }
    }

    /// Clears a branch, removing all of its patches.
    pub fn clear(&mut self, branch: &str) -> Result<(), Error> {
        let inode = self.inode(branch)?;
        self.storage.branch_patches.remove_all(branch);
        self.storage.remove_graggle(inode);
        self.storage
            .set_graggle(inode, storage::graggle::GraggleData::new());
        Ok(())
    }

    /// Persists the repository to disk.
    ///
    /// Any modifications that were previously made become permanent.
    pub fn write(&self) -> Result<(), Error> {
        let db = DbRef {
            current_branch: &self.current_branch,
            storage: &self.storage,
        };
        self.try_create_dir(&self.repo_dir)?;
        let db_file = fs::File::create(&self.db_path)?;
        serde_yaml::to_writer(db_file, &db)?;
        Ok(())
    }

    fn inode(&self, branch: &str) -> Result<storage::INode, Error> {
        self.storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))
    }

    /// Returns a read-only view to the data associated with a branch.
    pub fn graggle<'a>(&'a self, branch: &str) -> Result<storage::Graggle<'a>, Error> {
        let inode = self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?;
        Ok(self.storage.graggle(inode))
    }

    /// Retrieves the data associated with a branch, assuming that it represents a totally ordered
    /// file.
    pub fn file(&self, branch: &str) -> Result<File, Error> {
        let inode = self.inode(branch)?;
        self.storage
            .graggle(inode)
            .as_live_graph()
            .linear_order()
            .map(|ref order| File::from_ids(order, &self.storage))
            .ok_or(Error::NotOrdered)
    }

    /// Retrieves the contents associated with a node.
    pub fn contents(&self, id: &NodeId) -> &[u8] {
        self.storage.contents(id)
    }

    /// Opens a patch.
    ///
    /// The patch must already be known to the repository, either because it was created locally
    /// (i.e. with [`Repo::create_patch`]) or because it was (possibly created elsewhere but)
    /// registered locally with [`Repo::register_patch`].
    pub fn open_patch(&self, id: &PatchId) -> Result<Patch, Error> {
        let patch_data = self.open_patch_data(id)?;
        let ret = Patch::from_reader(patch_data)?;
        if ret.id() != id {
            Err(Error::IdMismatch(*ret.id(), *id))
        } else {
            Ok(ret)
        }
    }

    /// Returns the data associated with a patch.
    ///
    /// Currently, this data consists of the patch's contents serialized as YAML, but that isn't
    /// guaranteed. What is guaranteed is that the return value of this function is of the same
    /// format as the argument to [`Repo::register_patch`].
    pub fn open_patch_data(&self, id: &PatchId) -> Result<&[u8], Error> {
        self.storage
            .patches
            .get(id)
            .map(|s| s.as_bytes())
            .ok_or(Error::UnknownPatch(*id))
    }

    /// Introduces a patch to the repository.
    ///
    /// After registering a patch, its data will be stored in the repository and you will be able
    /// to access it by its ID.
    pub fn register_patch(&mut self, patch_data: &[u8]) -> Result<PatchId, Error> {
        let patch = Patch::from_reader(patch_data)?;
        let data = String::from_utf8(patch_data.to_owned())?;
        self.register_patch_with_data(&patch, data)?;
        Ok(*patch.id())
    }

    // Before making any modifications, check the patch for consistency. That means:
    // - all dependencies must already be known
    // - every node that we refer to must already be present
    // - every node that we refer to must be either new, or we must depend on its patch
    // This part is *IMPORTANT*, because it contains all the validation for patches. After
    // this, they go from being treated as untrusted input to being internal data.
    fn check_patch_validity(&self, patch: &Patch) -> Result<(), Error> {
        for dep in patch.deps() {
            if !self.storage.patches.contains_key(dep) {
                return Err(Error::MissingDep(*dep));
            }
        }
        let dep_set = patch.deps().iter().cloned().collect::<HashSet<_>>();
        let new_nodes = patch
            .changes()
            .changes
            .iter()
            .filter_map(|ch| {
                if let Change::NewNode { id, .. } = ch {
                    Some(id)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        for ch in &patch.changes().changes {
            use crate::patch::Change::*;
            let has_node = |id| {
                new_nodes.contains(id)
                    || (self.storage.contains_node(id) && dep_set.contains(&id.patch))
            };
            match ch {
                NewNode { id, .. } => {
                    if !has_node(id) {
                        return Err(Error::UnknownNode(*id));
                    }
                }
                NewEdge { src, dest } => {
                    if !has_node(src) {
                        return Err(Error::UnknownNode(*src));
                    }
                    if !has_node(dest) {
                        return Err(Error::UnknownNode(*dest));
                    }
                }
                DeleteNode { id } => {
                    if !has_node(id) {
                        return Err(Error::UnknownNode(*id));
                    }
                }
            }
        }
        Ok(())
    }

    fn register_patch_with_data(&mut self, patch: &Patch, data: String) -> Result<(), Error> {
        // If the patch already exists in our repository then there's nothing to do. But if there's
        // a file there with the same hash but different contents then something's really wrong.
        if self.storage.patches.contains_key(patch.id()) {
            let old_patch = self.open_patch(patch.id())?;
            if &old_patch == patch {
                return Ok(());
            } else {
                return Err(PatchIdError::Collision(*patch.id()).into());
            }
        }

        self.check_patch_validity(patch)?;

        // Record the deps and reverse-deps.
        for dep in patch.deps() {
            self.storage.patch_deps.insert(*patch.id(), *dep);
            self.storage.patch_rev_deps.insert(*dep, *patch.id());
        }

        self.storage.patches.insert(*patch.id(), data);
        Ok(())
    }

    // Applies a single patch to a branch.
    //
    // Panics if not all of the dependencies are already present.
    fn apply_one_patch(&mut self, branch: &str, patch_id: &PatchId) -> Result<(), Error> {
        let patch = self.open_patch(patch_id)?;
        for dep in patch.deps() {
            debug_assert!(
                self.storage.branch_patches.contains(branch, dep),
                "tried to apply a patch while it was missing a dependency"
            );
        }
        let inode = self.storage.inode(branch).unwrap();
        self.storage
            .apply_changes(inode, patch.changes(), *patch_id);
        self.storage
            .branch_patches
            .insert(branch.to_owned(), *patch.id());
        Ok(())
    }

    /// Applies a patch (and all its dependencies) to a branch.
    ///
    /// Returns a list of all the patches that were applied.
    pub fn apply_patch(&mut self, branch: &str, patch_id: &PatchId) -> Result<Vec<PatchId>, Error> {
        // If the branch already contains the patch, this is a no-op.
        if self.storage.branch_patches.contains(branch, patch_id) {
            return Ok(vec![]);
        }

        let mut patch_stack = vec![*patch_id];
        let mut applied = Vec::new();
        while !patch_stack.is_empty() {
            // The unwrap is ok because the stack is non-empty inside the loop.
            let cur = patch_stack.last().unwrap();
            let unapplied_deps = self
                .storage
                .patch_deps
                .get(cur)
                .filter(|dep| !self.storage.branch_patches.contains(branch, dep))
                .cloned()
                .collect::<Vec<_>>();
            if unapplied_deps.is_empty() {
                // It's possible that this patch was already applied, because it was a dep of
                // multiple other patches.
                if !self.storage.branch_patches.contains(branch, cur) {
                    self.apply_one_patch(branch, cur)?;
                    applied.push(*cur);
                }
                patch_stack.pop();
            } else {
                patch_stack.extend_from_slice(&unapplied_deps[..]);
            }
        }

        // Having applied all the patches, resolve the cache.
        let inode = self.storage.inode(branch).unwrap();
        self.storage.update_cache(inode);
        Ok(applied)
    }

    fn unapply_one_patch(&mut self, branch: &str, patch_id: &PatchId) -> Result<(), Error> {
        debug!("unapplying patch {:?} from branch {:?}", patch_id, branch);

        let patch = self.open_patch(patch_id)?;
        let inode = self.inode(branch)?;
        self.storage
            .unapply_changes(inode, patch.changes(), *patch_id);
        self.storage.branch_patches.remove(branch, patch.id());
        Ok(())
    }

    /// Unapplies a patch (and everything that depends on it) to a branch.
    ///
    /// Returns a list of all the patches that were unapplied.
    pub fn unapply_patch(
        &mut self,
        branch: &str,
        patch_id: &PatchId,
    ) -> Result<Vec<PatchId>, Error> {
        // If the branch doesn't contain the patch, this is a no-op.
        if !self.storage.branch_patches.contains(branch, patch_id) {
            return Ok(vec![]);
        }

        let mut patch_stack = vec![*patch_id];
        let mut unapplied = Vec::new();
        while !patch_stack.is_empty() {
            // The unwrap is ok because the stack is non-empty inside the loop.
            let cur = patch_stack.last().unwrap();
            let applied_rev_deps = self
                .storage
                .patch_rev_deps
                .get(cur)
                .filter(|dep| self.storage.branch_patches.contains(branch, dep))
                .cloned()
                .collect::<Vec<_>>();
            if applied_rev_deps.is_empty() {
                // It's possible that this patch was already unapplied, because it was a revdep of
                // multiple other patches.
                if self.storage.branch_patches.contains(branch, cur) {
                    self.unapply_one_patch(branch, cur)?;
                    unapplied.push(*cur);
                }
                patch_stack.pop();
            } else {
                patch_stack.extend_from_slice(&applied_rev_deps[..]);
            }
        }

        // Having unapplied all the patches, resolve the cache.
        let inode = self.storage.inode(branch).unwrap();
        self.storage.update_cache(inode);
        Ok(unapplied)
    }

    /// Returns an iterator over all known patches, applied or otherwise.
    pub fn all_patches(&self) -> impl Iterator<Item = &PatchId> {
        self.storage.patches.keys()
    }

    /// Returns an iterator over all of the patches being used in a branch.
    // TODO: maybe a way to check whether a patch is applied to a branch?
    pub fn patches(&self, branch: &str) -> impl Iterator<Item = &PatchId> + use<'_> {
        self.storage.branch_patches.get(branch)
    }

    /// Returns an iterator over all direct dependencies of the given patch.
    pub fn patch_deps(&self, patch: &PatchId) -> impl Iterator<Item = &PatchId> + use<'_> {
        self.storage.patch_deps.get(patch)
    }

    /// Returns an iterator over all direct dependents of the given patch.
    pub fn patch_rev_deps(&self, patch: &PatchId) -> impl Iterator<Item = &PatchId> + use<'_> {
        self.storage.patch_rev_deps.get(patch)
    }

    /// Creates a new patch with the given changes and metadata and returns its ID.
    ///
    /// The newly created patch will be automatically registered in the current repository, so
    /// there is no need to call [`Repo::register_patch`] on it.
    pub fn create_patch(
        &mut self,
        author: &str,
        msg: &str,
        changes: Changes,
    ) -> Result<PatchId, Error> {
        let patch = UnidentifiedPatch::new(author.to_owned(), msg.to_owned(), changes);

        // Serialize the patch to a buffer, and get back the identified patch.
        let mut patch_data = Vec::new();
        let patch = patch.write_out(&mut patch_data)?;
        let patch_data =
            String::from_utf8(patch_data).expect("YAML serializer failed to produce UTF-8");

        // Now that we know the patch's id, store it in the patches map.
        self.register_patch_with_data(&patch, patch_data)?;

        Ok(*patch.id())
    }

    fn try_create_dir(&self, dir: &Path) -> Result<(), Error> {
        if let Err(e) = std::fs::create_dir(dir) {
            // If the directory already exists, just swallow the error.
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e)?;
            }
        }
        Ok(())
    }

    /// Returns an iterator over the names of all branches.
    pub fn branches(&self) -> impl Iterator<Item = &str> {
        self.storage.branches()
    }

    /// Creates a new, empty branch.
    pub fn create_branch(&mut self, branch: &str) -> Result<(), Error> {
        if self.storage.inode(branch).is_some() {
            Err(Error::BranchExists(branch.to_owned()))
        } else {
            let inode = self.storage.allocate_inode();
            self.storage.set_inode(branch, inode);
            Ok(())
        }
    }

    /// Copies data to a new branch (which must not already exist).
    pub fn clone_branch(&mut self, from: &str, to: &str) -> Result<(), Error> {
        if self.storage.inode(to).is_some() {
            Err(Error::BranchExists(to.to_owned()))
        } else {
            let from_inode = self
                .storage
                .inode(from)
                .ok_or_else(|| Error::UnknownBranch(from.to_owned()))?;
            let to_inode = self.storage.clone_inode(from_inode);
            self.storage.set_inode(to, to_inode);

            // Record the fact that all the patches in the old branch are also present in the new
            // branch.
            let from_patches = self
                .storage
                .branch_patches
                .get(from)
                .cloned()
                .collect::<Vec<_>>();
            for p in from_patches {
                self.storage.branch_patches.insert(to.to_owned(), p);
            }
            Ok(())
        }
    }

    /// Deletes the branch named `branch`.
    pub fn delete_branch(&mut self, branch: &str) -> Result<(), Error> {
        if branch == self.current_branch {
            return Err(Error::CurrentBranch(branch.to_owned()));
        }
        let inode = self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?;
        self.storage.remove_graggle(inode);
        self.storage.remove_inode(branch);
        self.storage.branch_patches.remove_all(branch);
        Ok(())
    }

    /// Changes the current branch to the one named `branch` (which must already exist).
    pub fn switch_branch(&mut self, branch: &str) -> Result<(), Error> {
        if self.storage.inode(branch).is_none() {
            Err(Error::UnknownBranch(branch.to_owned()))
        } else {
            self.current_branch = branch.to_owned();
            Ok(())
        }
    }

    /// If the given branch represents a totally ordered file (i.e. if [`Repo::file`] returns
    /// something), returns the result of diffing the given branch against `file`.
    pub fn diff(&self, branch: &str, file: &[u8]) -> Result<Diff, Error> {
        let file_a = self.file(branch)?;
        let lines_a = (0..file_a.num_nodes())
            .map(|i| file_a.node(i))
            .collect::<Vec<_>>();

        let file_b = File::from_bytes(file);
        let lines_b = (0..file_b.num_nodes())
            .map(|i| file_b.node(i))
            .collect::<Vec<_>>();

        let diff = ojo_diff::diff(&lines_a, &lines_b);
        Ok(Diff {
            diff,
            file_a,
            file_b,
        })
    }
}

/// This struct, serialized, is the contents of the database.
#[derive(Debug, Deserialize, Serialize)]
struct Db {
    current_branch: String,
    storage: storage::Storage,
}

// The auto-generated Serialize implementation here should be compatible with the auto-generated
// Seserialize implementation for Db.
#[derive(Debug, Serialize)]
struct DbRef<'a> {
    current_branch: &'a str,
    storage: &'a storage::Storage,
}

/// Represents a diff between two [`File`](crate::File)s.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Diff {
    /// The first file.
    pub file_a: File,
    /// The second file.
    pub file_b: File,
    /// The diff going from `file_a` to `file_b`.
    pub diff: Vec<LineDiff>,
}
