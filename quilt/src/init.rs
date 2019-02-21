use clap::ArgMatches;
use failure::{Error, ResultExt};
use libquilt::Repo;

pub fn run(_m: &ArgMatches<'_>) -> Result<(), Error> {
    let dir = std::env::current_dir().context("Couldn't open the current directory.")?;
    let repo = Repo::init(&dir)?;
    repo.write()
        .context("Failed to write repository to disk.")?;
    eprintln!("Created empty quilt repository.");
    Ok(())
}
