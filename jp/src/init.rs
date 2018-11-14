use clap::ArgMatches;
use failure::{Error, ResultExt};
use libjp::Repo;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because "path" is a required argument.
    let path = m.value_of("PATH").unwrap();
    let repo = Repo::init(path)?;
    repo.write()
        .context("Failed to write repository to disk.")?;
    eprintln!("Created empty jp repository.");
    Ok(())
}
