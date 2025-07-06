use {anyhow::Result, clap::Parser};

#[derive(Debug, Parser)]
pub struct Opts {
    /// branch to clear (defaults to the current branch)
    branch: Option<String>,
}

pub fn run(opts: Opts) -> Result<()> {
    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, opts.branch);
    repo.clear(&branch)?;
    repo.write()?;
    Ok(())
}
