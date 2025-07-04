use {
    anyhow::{Context, Result},
    clap::Parser,
    libojo::PatchId,
    std::path::PathBuf,
};

#[derive(Parser, Debug)]
pub struct Opts {
    /// hash of the patch
    patch: String,
    /// path to the output file (defaults to the hash of the patch)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn run(opts: Opts) -> Result<()> {
    // The unwrap is ok because this is a required argument.
    let out = opts.output.unwrap_or(opts.patch.clone().into());

    let repo = crate::open_repo()?;
    let id = PatchId::from_base64(&opts.patch)?;
    let patch_data = repo.open_patch_data(&id)?;
    std::fs::write(&out, patch_data)
        .with_context(|| format!("Couldn't create file '{}'", out.display()))?;

    println!("Successfully wrote the file '{}'", out.display());
    Ok(())
}
