use serde_yaml;
use std::ffi::OsString;
use std::path::PathBuf;
use std::{self, fmt, io};

use crate::PatchId;

#[derive(Debug)]
pub enum Error {
    Base64Decode(base64::DecodeError),
    BranchExists(String),
    CurrentBranch(String),
    DbCorruption,
    Io(io::Error, String),
    MissingDep(PatchId),
    NoFilename(PathBuf),
    NoParent(PathBuf),
    NonUtfFilename(OsString),
    PatchCollision(crate::PatchId),
    RepoExists(PathBuf),
    RepoNotFound(PathBuf),
    Serde(serde_yaml::Error),
    UnknownBranch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CurrentBranch(b) => write!(f, "\"{}\" is the current branch", b),
            Error::DbCorruption => write!(f, "Found corruption in the database"),
            Error::Io(e, msg) => write!(f, "I/O error: {}. Details: {}", msg, e),
            Error::Base64Decode(e) => write!(f, "Error decoding base64: {}", e),
            Error::PatchCollision(id) => write!(
                f,
                "Encountered a collision between patch hashes: {}",
                base64::encode_config(&id.data[..], base64::URL_SAFE)
            ),
            Error::Serde(e) => e.fmt(f),
            Error::RepoNotFound(p) => write!(
                f,
                "I could not find a repository tracking this path: {:?}",
                p
            ),
            Error::RepoExists(p) => write!(f, "There is already a repository in {:?}", p),
            Error::MissingDep(id) => write!(f, "Missing a dependency: {}", id.filename()),
            Error::NoParent(p) => write!(f, "I could not find the parent directory of: {:?}", p),
            Error::NoFilename(p) => write!(f, "This path didn't end in a filename: {:?}", p),
            Error::NonUtfFilename(p) => {
                write!(f, "This filename couldn't be converted to UTF-8: {:?}", p)
            }
            Error::UnknownBranch(b) => write!(f, "There is no branch named {:?}", b),
            Error::BranchExists(b) => write!(f, "The branch \"{}\" already exists", b),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::BranchExists(_) => "branch already exists",
            Error::CurrentBranch(_) => "current branch",
            Error::DbCorruption => "found corruption in the database",
            Error::Io(e, _) => e.description(),
            Error::Base64Decode(e) => e.description(),
            Error::PatchCollision(_) => "patch collision detected",
            Error::Serde(e) => e.description(),
            Error::RepoNotFound(_) => "repository not found",
            Error::RepoExists(_) => "repository exists",
            Error::MissingDep(_) => "missing patch dependency",
            Error::NoParent(_) => "no parent path",
            Error::NoFilename(_) => "no filename",
            Error::NonUtfFilename(_) => "filename not UTF-8",
            Error::UnknownBranch(_) => "unknown branch",
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e, "".to_owned())
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Error {
        Error::Base64Decode(e)
    }
}

impl From<(io::Error, &'static str)> for Error {
    fn from((e, msg): (io::Error, &'static str)) -> Error {
        Error::Io(e, msg.to_owned())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Error {
        Error::Serde(e)
    }
}
