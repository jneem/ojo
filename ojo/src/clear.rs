use {anyhow::Result, clap::ArgMatches};

pub fn run(m: &ArgMatches<'_>) -> Result<()> {
    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, m);
    repo.clear(&branch)?;
    repo.write()?;
    Ok(())
}
