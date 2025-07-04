use {anyhow::Result, clap::ArgMatches, libojo::Changes};

pub fn run(m: &ArgMatches<'_>) -> Result<()> {
    // The unwraps are ok because these are required arguments.
    let msg = m.value_of("description").unwrap();
    let author = m.value_of("author").unwrap();

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);
    let path = crate::file_path(m);
    let diff = crate::diff::diff(&repo, &branch, &path)?;
    let changes = Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);
    let output_hash = m.is_present("output-hash");

    if changes.changes.is_empty() {
        if !output_hash {
            eprintln!("Not creating a patch because there were no changes.");
        }
        return Ok(());
    }

    let id = repo.create_patch(author, msg, changes)?;
    if m.is_present("then-apply") {
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
