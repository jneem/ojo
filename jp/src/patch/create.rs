use clap::ArgMatches;
use failure::Error;
use libjp::Changes;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwraps are ok because these are required arguments.
    let msg = m.value_of("description").unwrap();
    let author = m.value_of("author").unwrap();

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);
    let path = crate::file_path(m);
    let diff = crate::diff::diff(&repo, &branch, &path)?;
    let changes = Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);

    // TODO: we need a better error message if the file can't be opened (right now it just shows
    // the I/O error)

    if changes.changes.is_empty() {
        eprintln!("Not creating a patch because there were no changes.");
        return Ok(());
    }

    let id = repo.create_patch(author, msg, changes)?;
    repo.write()?;
    eprintln!("Created patch {}", id.to_base64());

    /*
    } else {
        // There was an error rendering the target branch to a file. In order to print an
        // informative message, we need to check whether the reason for failure was that the branch
        // doesn't exist, or whether the branch didn't have a linear order.
        if repo.graggle(&branch).is_ok() {
            bail!("Couldn't create a patch, because you need to resolve a conflict first.");
        } else {
            bail!("Couldn't open branch \"{}\"", branch);
        }
    };
    */
    Ok(())
}
