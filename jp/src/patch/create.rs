use clap::ArgMatches;
use failure::Error;
use libjp::Changes;
use std::io::prelude::*;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwraps are ok because these are required arguments.
    let msg = m.value_of("description").unwrap();
    let author = m.value_of("author").unwrap();

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);
    let path = crate::file_path(m);
    let diff = crate::diff::diff(&repo, &path)?;

    // TODO: we need a better error message if the file can't be opened (right now it just shows
    // the I/O error)
    // TODO: this is not very efficient: we're reading the file twice.
    let mut f = crate::open_file(&repo, &path)?;
    let mut contents = Vec::new();
    f.read_to_end(&mut contents)?;
    let new_file = libjp::File::from_bytes(&contents);
    if let Some(old_file) = repo.file(&branch) {
        let changes = Changes::from_diff(&old_file, &new_file, &diff.changes);

        if changes.changes.is_empty() {
            eprintln!("Not creating a patch because there were no changes.");
            return Ok(());
        }

        let id = repo.create_patch(author, msg, changes)?;
        repo.write()?;
        eprintln!("Created patch {}", id.to_base64());
    } else {
        // There was an error rendering the target branch to a file. In order to print an
        // informative message, we need to check whether the reason for failure was that the branch
        // doesn't exist, or whether the branch didn't have a linear order.
        if repo.digle(&branch).is_ok() {
            bail!("Couldn't create a patch, because you need to resolve a conflict first.");
        } else {
            bail!("Couldn't open branch \"{}\"", branch);
        }
    };
    Ok(())
}
