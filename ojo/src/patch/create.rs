use {anyhow::Result, clap::Parser, libojo::Changes, std::path::PathBuf};

#[derive(Parser, Debug)]
pub struct Opts {
    /// message describing the patch
    #[arg(short('m'), long)]
    description: String,
    /// the author of the patch
    #[arg(short, long)]
    author: String,
    /// branch to compare against (defaults to the current branch)
    #[arg(short, long)]
    branch: Option<String>,
    /// path to the file
    #[arg(default_value = "ojo_file.txt")]
    path: PathBuf,
    /// prints the hash value of the newly created patch to stdout
    #[arg(short('p'), default_value_t = false, long = "output-hash")]
    output_hash: bool,
    /// after creating the patch, apply it
    #[arg(short, long = "then-apply")]
    then_apply: bool,
}

pub fn run(opts: Opts) -> Result<()> {
    let msg = opts.description;
    let author = opts.author;

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, opts.branch);
    let path = opts.path;
    let diff = crate::diff::diff(&repo, &branch, &path)?;
    let changes = Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);
    let output_hash = opts.output_hash;

    if changes.changes.is_empty() {
        if !output_hash {
            eprintln!("Not creating a patch because there were no changes.");
        }
        return Ok(());
    }

    let id = repo.create_patch(&author, &msg, changes)?;
    if opts.then_apply {
        repo.apply_patch(&branch, &id)?;
        repo.write()?;
        if !output_hash {
            eprintln!("Created and applied patch {}", id.to_base64());
        }
    } else {
        repo.write()?;
        if !output_hash {
            eprintln!("Created patch {}", id.to_base64());
        }
    }

    if output_hash {
        println!("{}", id.to_base64());
    }
    Ok(())
}
