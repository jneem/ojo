use {
    anyhow::{Context, Result},
    clap::ArgMatches,
    libojo::Repo,
};

pub fn run(_m: &ArgMatches<'_>) -> Result<()> {
    let dir = std::env::current_dir().context("Couldn't open the current directory.")?;
    let repo = Repo::init(&dir)?;
    repo.write()
        .context("Failed to write repository to disk.")?;
    eprintln!("Created empty ojo repository.");
    Ok(())
}
