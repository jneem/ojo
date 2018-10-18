use clap::ArgMatches;
use libjp::Repo;
use std::io::prelude::*;
use std::fs::File;

pub fn run(m: &ArgMatches) -> Result<(), libjp::Error> {
    // The unwrap is ok because "path" is a required argument.
    let path = m.value_of("path").unwrap();
    let repo = Repo::open(path)?;

    unimplemented!();
    Ok(())
}
