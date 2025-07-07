use {anyhow::Result, clap::Parser};

#[derive(Parser, Debug)]
pub struct Opts {
    /// branch whose patches we want to print (defaults to the current branch)
    #[arg(short, long)]
    branch: Option<String>,
}

pub fn run(opts: Opts) -> Result<()> {
    let repo = super::open_repo()?;
    let branch = super::branch(&repo, opts.branch);

    for patch_id in repo.patches(&branch) {
        let patch = repo.open_patch(patch_id)?;
        println!("patch {}", patch_id.to_base64());
        println!("Author: {}", patch.header().author);
        println!();
        // TODO: dates and sorting.
        // TODO: better display for multi-line description.
        println!("\t{}", patch.header().description);
        println!();
    }
    Ok(())
}
