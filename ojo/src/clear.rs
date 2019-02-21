use clap::ArgMatches;
use failure::Error;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, m);
    repo.clear(&branch)?;
    repo.write()?;
    Ok(())
}
