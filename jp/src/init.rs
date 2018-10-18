use clap::ArgMatches;
use libjp::Repo;

pub fn run(m: &ArgMatches) -> Result<(), libjp::Error> {
    // The unwrap is ok because "path" is a required argument.
    let path = m.value_of("PATH").unwrap();
    let repo = Repo::init(path)?;
    repo.write()
}

