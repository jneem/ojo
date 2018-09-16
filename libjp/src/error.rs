use serde_yaml;
use std::{self, fmt, io};
use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Serde(serde_yaml::Error),
    RepoNotFound(PathBuf),
    RepoExists(PathBuf),
    NoParent(PathBuf),
    NoFilename(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Serde(e) => e.fmt(f),
            Error::RepoNotFound(p) =>
                write!(f, "I could not find a repository tracking this path: {:?}", p),
            Error::RepoExists(p) =>
                write!(f, "There is already a repository tracking this path: {:?}", p),
            Error::NoParent(p) =>
                write!(f, "I could not find the parent directory of: {:?}", p),
            Error::NoFilename(p) =>
                write!(f, "This path didn't end in a filename: {:?}", p),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::Io(e) => e.description(),
            Error::Serde(e) => e.description(),
            Error::RepoNotFound(_) => "repository not found",
            Error::RepoExists(_) => "repository exists",
            Error::NoParent(_) => "no parent path",
            Error::NoFilename(_) => "no filename",
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Error {
        Error::Serde(e)
    }
}
