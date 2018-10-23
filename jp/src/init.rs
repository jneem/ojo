use clap::ArgMatches;
use failure::Error;
use libjp::Repo;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok because "path" is a required argument.
    let path = m.value_of("PATH").unwrap();
    let repo = Repo::init(path)?;
    repo.write()?;
    Ok(())
}

