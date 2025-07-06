use {anyhow::Result, clap::Parser, libojo::PatchId};

#[derive(Parser, Debug)]
pub struct Opts {
    /// hash of the patch
    patch: String,
    /// branch to apply the patch to (defaults to the current branch)
    #[arg(short, long)]
    branch: Option<String>,
    /// if set, unapplies the patch instead of applying it
    #[arg(short('R'), long)]
    revert: bool,
}

pub fn run(opts: Opts) -> Result<()> {
    let patch_id = PatchId::from_base64(&opts.patch)?;

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, opts.branch);

    if opts.revert {
        let unapplied = repo.unapply_patch(&branch, &patch_id)?;
        if unapplied.is_empty() {
            eprintln!("No patches to unapply.");
        } else {
            println!("Unapplied:");
            for u in unapplied {
                println!("  {}", u.to_base64());
            }
        }
    } else {
        let applied = repo.apply_patch(&branch, &patch_id)?;
        if applied.is_empty() {
            eprintln!("No patches to apply.");
        } else {
            println!("Applied:");
            for a in applied {
                println!("  {}", a.to_base64());
            }
        }
    }

    repo.write()?;
    Ok(())
}
