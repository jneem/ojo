use clap::ArgMatches;
use colored::*;
use failure::{Error, Fail};
use libojo::Repo;
use ojo_diff::LineDiff;
use std::fmt;

pub struct DiffDisplay(pub libojo::Diff);

impl fmt::Display for DiffDisplay {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &ch in &self.0.diff {
            match ch {
                LineDiff::New(i) => {
                    let s = format!("+ {}", String::from_utf8_lossy(&self.0.file_b.node(i)));
                    write!(fmt, "{}", s.green())?;
                }
                LineDiff::Delete(i) => {
                    let s = format!("- {}", String::from_utf8_lossy(&self.0.file_a.node(i)));
                    write!(fmt, "{}", s.red())?;
                }
                LineDiff::Keep(i, _) => {
                    write!(fmt, "  {}", String::from_utf8_lossy(&self.0.file_a.node(i)))?;
                }
            }
        }
        Ok(())
    }
}

pub fn diff(repo: &Repo, branch: &str, file_name: &str) -> Result<libojo::Diff, Error> {
    let mut path = repo.root_dir.clone();
    path.push(file_name);
    let fs_file_contents = std::fs::read(&path)
        .map_err(|e| e.context(format!("Could not read the file {}", file_name)))?;

    let ret = repo.diff(branch, &fs_file_contents[..]).map_err(|e| {
        if let libojo::Error::NotOrdered = e {
            e.context(format!(
                "Cannot create a diff because the repo's contents aren't ordered"
            ))
            .into()
        } else {
            Error::from(e)
        }
    });
    Ok(ret?)
}

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let repo = super::open_repo()?;
    let branch = super::branch(&repo, m);
    let file_name = super::file_path(m);

    let diff = diff(&repo, &branch, &file_name)?;
    print!("{}", DiffDisplay(diff));

    Ok(())
}
