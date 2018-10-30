use clap::ArgMatches;
use failure::{Error, ResultExt};
use libjp::patch::UnidentifiedPatch;
use libjp::storage;
use libjp::Changes;
use std::io::prelude::*;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    // The unwraps are ok because these are required arguments.
    let msg = m.value_of("description").unwrap();
    let author = m.value_of("author").unwrap();

    let repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);
    let diff = crate::diff::diff(&repo)?;

    // TODO: this is not very efficient: we're reading the file twice.
    let mut f = repo.open_file()?;
    let mut contents = Vec::new();
    f.read_to_end(&mut contents)?;
    let new_file = storage::File::from_bytes(&contents);
    if let Some(old_file) = repo.file(&branch) {
        let changes = Changes::from_diff(&old_file, &new_file, &diff);

        if changes.changes.is_empty() {
            eprintln!("Not creating a patch because there were no changes.");
            return Ok(());
        }

        let patch = UnidentifiedPatch::new(author.to_owned(), msg.to_owned(), changes);

        // Write the patch to a temporary file, and get back the identified patch.
        let mut out =
            tempfile::NamedTempFile::new_in(".").context("trying to create a named temp file")?;
        let patch = patch.write_out(&mut out)?;

        // Now that we know the patch's id, move it to a location given by that name.
        out.persist(&patch.id.filename())
            .with_context(|_| format!("saving patch to {}", &patch.id.filename()))?;
        eprintln!("Saved patch to: {}", &patch.id.filename());
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
