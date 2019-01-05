// Allow missing docs in this module, for now, because we need to think more about the types of
// errors we're exposing.
#![allow(missing_docs)]

use serde_yaml;
use std::ffi::OsString;
use std::path::PathBuf;
use std::{self, fmt, io};

use crate::PatchId;

#[derive(Debug)]
pub enum PatchIdError {
    Base64Decode(base64::DecodeError),
    InvalidLength(usize),
    Collision(crate::PatchId),
}

impl From<base64::DecodeError> for PatchIdError {
    fn from(e: base64::DecodeError) -> PatchIdError {
        PatchIdError::Base64Decode(e)
    }
}

impl fmt::Display for PatchIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::PatchIdError::*;

        match self {
            Base64Decode(e) => e.fmt(f),
            InvalidLength(n) => write!(f, "Found the wrong number of bytes: {}", n),
            Collision(p) => write!(
                f,
                "Encountered a collision between patch hashes: {}",
                p.to_base64()
            ),
        }
    }
}

impl std::error::Error for PatchIdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::PatchIdError::*;

        match self {
            Base64Decode(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    PatchId(PatchIdError),
    IdMismatch(PatchId, PatchId),
    BranchExists(String),
    CurrentBranch(String),
    DbCorruption,
    Io(io::Error, String),
    MissingDep(PatchId),
    NoFilename(PathBuf),
    NoParent(PathBuf),
    NonUtfFilename(OsString),
    PersistError(tempfile::PersistError),
    RepoExists(PathBuf),
    RepoNotFound(PathBuf),
    Serde(serde_yaml::Error),
    UnknownBranch(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CurrentBranch(b) => write!(f, "\"{}\" is the current branch", b),
            Error::DbCorruption => write!(f, "Found corruption in the database"),
            Error::Io(e, msg) => write!(f, "I/O error: {}. Details: {}", msg, e),
            Error::PatchId(_) => write!(f, "Found a broken PatchId"),
            Error::IdMismatch(actual, expected) => write!(
                f,
                "Expected {}, found {}",
                expected.to_base64(),
                actual.to_base64()
            ),
            Error::PersistError(e) => e.fmt(f),
            Error::Serde(e) => e.fmt(f),
            Error::RepoNotFound(p) => write!(
                f,
                "I could not find a repository tracking this path: {:?}",
                p
            ),
            Error::RepoExists(p) => write!(f, "There is already a repository in {:?}", p),
            Error::MissingDep(id) => write!(f, "Missing a dependency: {}", id.to_base64()),
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
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e, _) => Some(e),
            Error::PatchId(e) => Some(e),
            Error::PersistError(e) => Some(e),
            Error::Serde(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PatchIdError> for Error {
    fn from(e: PatchIdError) -> Error {
        Error::PatchId(e)
    }
}

impl From<tempfile::PersistError> for Error {
    fn from(e: tempfile::PersistError) -> Error {
        Error::PersistError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e, "".to_owned())
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
