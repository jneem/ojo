#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate proptest;

use std::fs;
use std::path::{Path, PathBuf};

mod error;
mod patch;
pub mod resolver;
mod storage;

pub use crate::error::{Error, PatchIdError};
pub use crate::patch::{Change, Changes, Patch, PatchId, UnidentifiedPatch};
pub use crate::storage::{Digle, File};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NodeId {
    pub patch: PatchId,
    pub node: u64,
}

impl NodeId {
    fn set_patch_id(&mut self, id: &PatchId) {
        if self.patch.is_cur() {
            self.patch = id.clone();
        }
    }

    fn cur(node: u64) -> NodeId {
        NodeId {
            patch: PatchId::cur(),
            node: node,
        }
    }
}

#[derive(Debug)]
pub struct Repo {
    /// The path to the root directory of the repository.
    pub root_dir: PathBuf,
    /// The path to the directory where all of jp's data is stored.
    pub repo_dir: PathBuf,
    /// The path to the database containing all the history, and so on.
    pub db_path: PathBuf,
    /// The path to the directory where patches are stored.
    pub patch_dir: PathBuf,
    /// The name of the file that is being tracked.
    pub file_name: String,
    pub current_branch: String,

    storage: storage::Storage,
}

impl Repo {
    /// Given the path of the root directory of a repository, returns the directory where jp's data
    /// is stored.
    fn repo_dir(dir: &Path) -> Result<PathBuf, Error> {
        let mut ret = dir.to_path_buf();
        ret.push(".jp");
        Ok(ret)
    }

    /// Given the path of the root directory of a repository, returns the path containing jp's
    /// serialized data.
    fn db_path(dir: &Path) -> Result<PathBuf, Error> {
        let mut ret = Repo::repo_dir(dir)?;
        ret.push("db");
        Ok(ret)
    }

    /// Given the path of the root directory of a repository, returns the path containing all the
    /// patches contained in the repository.
    fn patch_dir(file_path: &Path) -> Result<PathBuf, Error> {
        let mut ret = Repo::repo_dir(file_path)?;
        ret.push("patches");
        Ok(ret)
    }

    pub fn open_file(&self) -> Result<fs::File, Error> {
        let mut path = self.root_dir.clone();
        path.push(&self.file_name);
        Ok(fs::File::open(path)?)
    }

    /// Opens the existing repository with the given root directory.
    pub fn open<P: AsRef<Path>>(dir: P) -> Result<Repo, Error> {
        let db_path = Repo::db_path(dir.as_ref())?;
        let patch_dir = Repo::patch_dir(dir.as_ref())?;
        let db_file = fs::File::open(&db_path)?;
        let db: Db = serde_yaml::from_reader(db_file)?;
        Ok(Repo {
            root_dir: dir.as_ref().to_owned(),
            repo_dir: Repo::repo_dir(dir.as_ref())?,
            db_path,
            patch_dir,
            file_name: db.file_name,
            current_branch: db.current_branch,
            storage: db.storage,
        })
    }

    /// Creates a repo for tracking the given file.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Repo, Error> {
        let root_dir = path
            .as_ref()
            .parent()
            .ok_or_else(|| Error::NoParent(path.as_ref().to_owned()))?
            .to_owned();
        let repo_dir = Repo::repo_dir(&root_dir)?;
        let db_path = Repo::db_path(&root_dir)?;
        if db_path.exists() {
            return Err(Error::RepoExists(repo_dir.clone()));
        }
        let patch_dir = Repo::patch_dir(&root_dir)?;
        let file_name = path
            .as_ref()
            .file_name()
            .ok_or_else(|| Error::NoFilename(path.as_ref().to_owned()))?;
        let file_name = file_name
            .to_str()
            .ok_or_else(|| Error::NonUtfFilename(file_name.to_owned()))?;

        let mut storage = storage::Storage::new();
        let master_inode = storage.allocate_inode();
        storage.set_inode("master", master_inode);
        Ok(Repo {
            root_dir,
            repo_dir,
            db_path: db_path,
            patch_dir: patch_dir,
            file_name: file_name.to_owned(),
            current_branch: "master".to_owned(),
            storage: storage,
        })
    }

    /// Clears a branch, removing all of its patches.
    pub fn clear(&mut self, branch: &str) -> Result<(), Error> {
        let inode = self.inode(branch)?;
        self.storage.branch_patches.remove_all(branch);
        self.storage.remove_digle(inode);
        self.storage.set_digle(inode, storage::digle::DigleData::new());
        Ok(())
    }

    pub fn write(&self) -> Result<(), Error> {
        let db = DbRef {
            file_name: &self.file_name,
            current_branch: &self.current_branch,
            storage: &self.storage,
        };
        self.try_create_dir(&self.repo_dir)?;
        self.try_create_dir(&self.patch_dir)?;
        let db_file = fs::File::create(&self.db_path)?;
        serde_yaml::to_writer(db_file, &db)?;
        Ok(())
    }

    pub fn inode(&self, branch: &str) -> Result<storage::INode, Error> {
        Ok(self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?)
    }

    pub fn digle<'a>(&'a self, branch: &str) -> Result<storage::Digle<'a>, Error> {
        let inode = self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?;
        Ok(self.storage.digle(inode))
    }

    pub fn digle_mut<'a>(&'a mut self, branch: &str) -> Result<storage::DigleMut<'a>, Error> {
        let inode = self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?;
        Ok(self.storage.digle_mut(inode))
    }

    pub fn file(&self, branch: &str) -> Option<File> {
        use graph::Graph;
        let inode = self.storage.inode(branch)?;
        self.storage
            .digle(inode)
            .linear_order()
            .map(|order| File::from_ids(&order, &self.storage))
    }

    pub fn contents(&self, id: &NodeId) -> &[u8] {
        self.storage.contents(id)
    }

    fn patch_path(&self, id: &PatchId) -> PathBuf {
        let mut ret = self.patch_dir.clone();
        ret.push(id.to_base64());
        ret
    }

    pub fn open_patch_by_id(&self, id: &PatchId) -> Result<Patch, Error> {
        let ret = Patch::from_reader(fs::File::open(self.patch_path(id))?)?;
        if ret.id() != id {
            Err(Error::IdMismatch(*ret.id(), *id))
        } else {
            Ok(ret)
        }
    }

    pub fn open_patch(&self, name: &str) -> Result<Patch, Error> {
        self.open_patch_by_id(&PatchId::from_base64(name)?)
    }

    pub fn register_patch(&mut self, patch: &Patch) -> Result<(), Error> {
        // If the patch already exists in our repository then there's nothing to do. But if there's
        // a file there which doesn't match this one then something's really wrong.
        if self.storage.patches.contains(patch.id()) {
            let old_patch = self.open_patch_by_id(patch.id())?;
            if &old_patch == patch {
                return Ok(());
            } else {
                return Err(PatchIdError::Collision(patch.id().clone()).into());
            }
        }

        // Record the deps and reverse-deps.
        for dep in patch.deps() {
            if !self.storage.patches.contains(dep) {
                return Err(Error::MissingDep(dep.clone()));
            }
            self.storage
                .patch_deps
                .insert(patch.id().clone(), dep.clone());
            self.storage
                .patch_rev_deps
                .insert(dep.clone(), patch.id().clone());
        }

        self.storage.patches.insert(patch.id().clone());
        Ok(())
    }

    // Applies a single patch to a branch.
    //
    // Panics if not all of the dependencies are already present.
    fn apply_one_patch(&mut self, branch: &str, patch_id: &PatchId) -> Result<(), Error> {
        let patch = self.open_patch_by_id(patch_id)?;
        for dep in patch.deps() {
            debug_assert!(
                self.storage.branch_patches.contains(branch, dep),
                "tried to apply a patch while it was missing a dependency"
            );
        }
        let inode = self.storage.inode(branch).unwrap();
        self.storage.apply_changes(inode, patch.changes());
        self.storage.patches.insert(patch.id().clone());
        self.storage
            .branch_patches
            .insert(branch.to_owned(), patch.id().clone());
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

        let mut patch_stack = vec![patch_id.clone()];
        let mut applied = Vec::new();
        while !patch_stack.is_empty() {
            // The unwrap is ok because the stack is non-empty inside the loop.
            let cur = patch_stack.last().unwrap();
            let unapplied_deps = self
                .storage
                .patch_deps
                .get(&cur)
                .filter(|dep| !self.storage.branch_patches.contains(branch, dep))
                .cloned()
                .collect::<Vec<_>>();
            if unapplied_deps.is_empty() {
                self.apply_one_patch(branch, &cur)?;
                applied.push(cur.clone());
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
        let patch = self.open_patch_by_id(patch_id)?;
        let inode = self.inode(branch)?;
        self.storage.unapply_changes(inode, patch.changes());
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

        let mut patch_stack = vec![patch_id.clone()];
        let mut unapplied = Vec::new();
        while !patch_stack.is_empty() {
            // The unwrap is ok because the stack is non-empty inside the loop.
            let cur = patch_stack.last().unwrap();
            let applied_rev_deps = self
                .storage
                .patch_rev_deps
                .get(&cur)
                .filter(|dep| self.storage.branch_patches.contains(branch, dep))
                .cloned()
                .collect::<Vec<_>>();
            if applied_rev_deps.is_empty() {
                self.unapply_one_patch(branch, &cur)?;
                unapplied.push(cur.clone());
                patch_stack.pop();
            } else {
                patch_stack.extend_from_slice(&applied_rev_deps[..]);
            }
        }
        Ok(unapplied)
    }

    pub fn patches(&self, branch: &str) -> impl Iterator<Item = &PatchId> {
        self.storage.branch_patches.get(branch)
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

    pub fn branches(&self) -> impl Iterator<Item = &str> {
        self.storage.branches()
    }

    pub fn create_branch(&mut self, branch: &str) -> Result<(), Error> {
        if self.storage.inode(branch).is_some() {
            Err(Error::BranchExists(branch.to_owned()))
        } else {
            let inode = self.storage.allocate_inode();
            self.storage.set_inode(branch, inode);
            Ok(())
        }
    }

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

    pub fn delete_branch(&mut self, branch: &str) -> Result<(), Error> {
        if branch == &self.current_branch {
            return Err(Error::CurrentBranch(branch.to_owned()));
        }
        let inode = self
            .storage
            .inode(branch)
            .ok_or_else(|| Error::UnknownBranch(branch.to_owned()))?;
        self.storage.remove_digle(inode);
        self.storage.remove_inode(branch);
        self.storage.branch_patches.remove_all(branch);
        Ok(())
    }

    pub fn switch_branch(&mut self, branch: &str) -> Result<(), Error> {
        if self.storage.inode(branch).is_none() {
            Err(Error::UnknownBranch(branch.to_owned()))
        } else {
            self.current_branch = branch.to_owned();
            Ok(())
        }
    }
}

/// This struct, serialized, is the contents of the database.
#[derive(Debug, Deserialize, Serialize)]
struct Db {
    // Path (relative to the repository's root directory) to the file being tracked.
    file_name: String,
    current_branch: String,
    storage: storage::Storage,
}

// I *think* that the auto-generated Serialize implementation here is compatible with the
// auto-generated Seserialize implementation for Db.
#[derive(Debug, Serialize)]
struct DbRef<'a> {
    file_name: &'a str,
    current_branch: &'a str,
    storage: &'a storage::Storage,
}
