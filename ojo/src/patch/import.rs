use {
    anyhow::{Context, Result},
    clap::Parser,
    std::path::PathBuf,
};

#[derive(Parser, Debug)]
pub struct Opts {
    /// path to the patch file
    path: PathBuf,
}

pub fn run(opts: Opts) -> Result<()> {
    let mut repo = crate::open_repo()?;
    let contents = std::fs::read(&opts.path)
        .with_context(|| format!("Failed to read file '{}'", opts.path.display()))?;
    let id = repo.register_patch(&contents)?;
    repo.write()?;

    println!("Successfully imported a patch with id {}", id.to_base64());
    Ok(())
}
