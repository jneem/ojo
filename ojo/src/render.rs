use {
    anyhow::{Result, anyhow},
    clap::Parser,
    std::path::PathBuf,
};

#[derive(Parser, Debug)]
pub struct Opts {
    /// branch to output (defaults to the current branch)
    #[arg(short, long)]
    branch: Option<String>,
    /// path of the output
    #[arg(default_value = "ojo_file.txt")]
    path: PathBuf,
}

pub fn run(opts: Opts) -> Result<()> {
    let repo = crate::open_repo()?;
    let branch = crate::branch(&repo, opts.branch);
    let file = repo.file(&branch).map_err(|e| match e {
        libojo::Error::NotOrdered => {
            anyhow!("Couldn't render a file, because the data isn't ordered")
        }
        other => other.into(),
    })?;

    std::fs::write(&opts.path, file.as_bytes())?;
    eprintln!("Successfully wrote file '{}'", opts.path.display());

    Ok(())
}
