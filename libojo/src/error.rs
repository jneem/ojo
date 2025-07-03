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

// Allow missing docs in this module, for now, because we need to think more about the types of
// errors we're exposing.
#![allow(missing_docs)]

use std::{self, ffi::OsString, io, path::PathBuf};

use crate::{NodeId, PatchId};

#[derive(thiserror::Error, Debug)]
pub enum PatchIdError {
    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Found the wrong number of bytes: {0}")]
    InvalidLength(usize),
    #[error("Encountered a collision between patch hashes: {}", .0.to_base64())]
    Collision(crate::PatchId),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The branch \"{0}\" already exists")]
    BranchExists(String),
    #[error("\"{0}\" is the current branch")]
    CurrentBranch(String),
    #[error("Found corruption in the database")]
    DbCorruption,
    #[error(transparent)]
    Encoding(#[from] std::string::FromUtf8Error),
    #[error("Expected {}, found {}", .1.to_base64(), .0.to_base64())]
    IdMismatch(PatchId, PatchId),
    #[error("I/O error: {}. Details: {}", .0, .1)]
    Io(io::Error, String),
    #[error("Missing a dependency: {}", .0.to_base64())]
    MissingDep(PatchId),
    #[error("This path didn't end in a filename: {0:?}")]
    NoFilename(PathBuf),
    #[error("I could not find the parent directory of: {0:?}")]
    NoParent(PathBuf),
    #[error("This filename couldn't be converted to UTF-8: {0:?}")]
    NonUtfFilename(OsString),
    #[error("The data does not represent a totally ordered file")]
    NotOrdered,
    #[error("Found a broken PatchId\n\tcaused by: {0}")]
    PatchId(#[from] PatchIdError),
    #[error("There is already a repository in {0:?}")]
    RepoExists(PathBuf),
    #[error("I could not find a repository tracking this path: {0:?}")]
    RepoNotFound(PathBuf),
    #[error(transparent)]
    Serde(#[from] serde_yaml::Error),
    #[error("There is no branch named {0:?}")]
    UnknownBranch(String),
    #[error("There is no node with id {0:?}")]
    UnknownNode(NodeId),
    #[error("There is no patch with hash {:?}", .0.to_base64())]
    UnknownPatch(PatchId),
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
