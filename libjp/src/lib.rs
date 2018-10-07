#![feature(nll)]

#[macro_use]
extern crate serde_derive;

extern crate itertools;
extern crate multimap;
extern crate rpds;
extern crate serde_yaml;
extern crate sha2;

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

mod error;
pub mod graph;
pub mod patch;
pub mod storage;

pub use error::Error;
pub use patch::{Patch, PatchId, UnidentifiedPatch};
pub use storage::Digle;

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
    /// Given the name of a file that is being versioned, returns the path containing its
    /// serialized digle.
    fn db_path(file_path: &Path) -> Result<PathBuf, Error> {
        let parent = file_path
            .parent()
            .ok_or_else(|| Error::NoParent(file_path.to_path_buf()))?;
        let file_name = file_path
            .file_name()
            .ok_or_else(|| Error::NoFilename(file_path.to_path_buf()))?;

        let mut ret = parent.to_path_buf();
        let mut db_file_name = OsString::from("db_");
        db_file_name.push(file_name);
        ret.push(".jp");
        ret.push(db_file_name);
        Ok(ret)
    }

    /// Given the name of a file that is being versioned, returns the directory containing all the
    /// patches related to that file.
    fn patch_dir(file_path: &Path) -> Result<PathBuf, Error> {
        let parent = file_path
            .parent()
            .ok_or_else(|| Error::NoParent(file_path.to_path_buf()))?;

        let mut ret = parent.to_path_buf();
        ret.push(".jp");
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

        Ok(Repo {
            db_path: db_path,
            patch_dir: patch_dir,
            file: path.as_ref().to_path_buf(),
            storage: storage::Storage::new(),
            patches: HashSet::new(),
        })
    }

    pub fn storage(&self) -> &storage::Storage {
        &self.storage
    }

    pub fn file(&self) -> Option<Vec<u8>> {
        unimplemented!();
    }
}

/// This struct, serialized, is the contents of the database.
#[derive(Debug, Deserialize, Serialize)]
struct Db {
    storage: storage::Storage,
    patches: HashSet<PatchId>,
}
