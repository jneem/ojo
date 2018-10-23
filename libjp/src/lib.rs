#[macro_use] extern crate serde_derive;

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod error;
pub mod graph;
pub mod patch;
pub mod storage;

pub use crate::error::Error;
pub use crate::patch::{Change, Changes, Patch, PatchId, UnidentifiedPatch};
pub use crate::storage::Digle;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LineId {
    patch: PatchId,
    line: u64,
}

impl LineId {
    fn set_patch_id(&mut self, id: &PatchId) {
        if self.patch.is_cur() {
            self.patch = id.clone();
        }
    }

    fn cur(line: u64) -> LineId {
        LineId {
            patch: PatchId::cur(),
            line: line,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    pub dest: LineId,
}

#[derive(Debug)]
pub struct Repo {
    /// The path to the database containing all the history, and so on.
    pub db_path: PathBuf,
    /// The path to the directory where patches are stored.
    pub patch_dir: PathBuf,
    /// The path to the file that is being tracked.
    pub file: PathBuf,

    storage: storage::Storage,
    patches: HashSet<PatchId>,
}

impl Repo {
    /// Given the name of a file that is being versioned, returns the path to the directory where
    /// everything is stored.
    fn repo_dir(file_path: &Path) -> Result<PathBuf, Error> {
        let parent = file_path
            .parent()
            .ok_or_else(|| Error::NoParent(file_path.to_path_buf()))?;
        let mut ret = parent.to_path_buf();
        ret.push(".jp");
        Ok(ret)
    }

    /// Given the name of a file that is being versioned, returns the path containing its
    /// serialized digle.
    fn db_path(file_path: &Path) -> Result<PathBuf, Error> {
        let file_name = file_path
            .file_name()
            .ok_or_else(|| Error::NoFilename(file_path.to_path_buf()))?;

        let mut ret = Repo::repo_dir(file_path)?;
        let mut db_file_name = OsString::from("db_");
        db_file_name.push(file_name);
        ret.push(db_file_name);
        Ok(ret)
    }

    /// Given the name of a file that is being versioned, returns the directory containing all the
    /// patches related to that file.
    fn patch_dir(file_path: &Path) -> Result<PathBuf, Error> {
        let mut ret = Repo::repo_dir(file_path)?;
        ret.push("patches");
        Ok(ret)
    }

    /// Opens the existing repo that is tracking the given file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Repo, Error> {
        let db_path = Repo::db_path(path.as_ref())?;
        let patch_dir = Repo::patch_dir(path.as_ref())?;
        let db_file = File::open(&db_path)?;
        let db: Db = serde_yaml::from_reader(db_file)?;
        Ok(Repo {
            db_path,
            patch_dir,
            file: path.as_ref().to_path_buf(),
            storage: db.storage,
            patches: db.patches,
        })
    }

    /// Creates a repo for tracking the given file.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Repo, Error> {
        let db_path = Repo::db_path(path.as_ref())?;
        if db_path.exists() {
            return Err(Error::RepoExists(db_path));
        }
        let patch_dir = Repo::patch_dir(path.as_ref())?;

        let mut storage = storage::Storage::new();
        let master_inode = storage.allocate_inode();
        storage.set_inode("master", master_inode);
        Ok(Repo {
            db_path: db_path,
            patch_dir: patch_dir,
            file: path.as_ref().to_path_buf(),
            storage: storage,
            patches: HashSet::new(),
        })
    }

    pub fn write(&self) -> Result<(), Error> {
        let db = DbRef {
            storage: &self.storage,
            patches: &self.patches,
        };
        self.try_create_dir(&Repo::repo_dir(&self.file)?)?;
        self.try_create_dir(&Repo::patch_dir(&self.file)?)?;
        let db_file = File::create(&self.db_path)?;
        serde_yaml::to_writer(db_file, &db)?;
        Ok(())
    }

    pub fn storage(&self) -> &storage::Storage {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut storage::Storage {
        &mut self.storage
    }

    pub fn file(&self, branch: &str) -> Option<storage::File> {
        use crate::graph::GraphRef;
        let inode = self.storage.inode(branch).unwrap(); // FIXME: unwrap
        self.storage.digle(inode).linear_order().map(|order| {
            storage::File::from_ids(&order, &self.storage)
        })
    }

    pub fn patches(&self) -> &HashSet<PatchId> {
        &self.patches
    }

    fn patch_path(&self, id: &PatchId) -> PathBuf {
        let mut ret = self.patch_dir.clone();
        ret.push(id.filename());
        ret
    }

    fn open_patch(&self, id: &PatchId) -> Result<Patch, Error> {
        Patch::from_reader(File::open(self.patch_path(id))?, id.clone())
    }

    pub fn register_patch(&mut self, patch: &Patch) -> Result<(), Error> {
        let patch_path = self.patch_path(&patch.id);

        // If the patch already exists in our repository then there's nothing to do. But if there's
        // a file there which doesn't match this one then something's really wrong.
        if self.patches.contains(&patch.id) {
            let old_patch = self.open_patch(&patch.id)?;
            if &old_patch == patch {
                return Ok(());
            } else {
                return Err(Error::PatchCollision(patch.id.clone()));
            }
        }

        let mut out = File::create(&patch_path)?;
        let id = patch.id.clone();
        patch.write_out(&mut out)?;
        self.patches.insert(id);
        Ok(())
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
}

/// This struct, serialized, is the contents of the database.
#[derive(Debug, Deserialize, Serialize)]
struct Db {
    storage: storage::Storage,
    patches: HashSet<PatchId>,
}

// I *think* that the auto-generated Serialize implementation here is compatible with the
// auto-generated Seserialize implementation for Db.
#[derive(Debug, Serialize)]
struct DbRef<'a> {
    storage: &'a storage::Storage,
    patches: &'a HashSet<PatchId>,
}
