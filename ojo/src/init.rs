use {
    anyhow::{Context, Result},
    libojo::Repo,
};

// no any Opts

pub fn run() -> Result<()> {
    let dir = std::env::current_dir().context("Couldn't open the current directory.")?;
    let repo = Repo::init(&dir)?;
    repo.write()
        .context("Failed to write repository to disk.")?;
    eprintln!("Created empty ojo repository.");
    Ok(())
}
