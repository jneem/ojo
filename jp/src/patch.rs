use clap::ArgMatches;
use libjp::{Changes, Repo};
use libjp::patch::UnidentifiedPatch;
use libjp::storage;
use std::io::prelude::*;
use std::fs::File;

pub fn run(m: &ArgMatches) -> Result<(), libjp::Error> {
    // The unwraps are ok because these are required arguments.
    let path = m.value_of("PATH").unwrap();
    let msg = m.value_of("description").unwrap();
    let author = m.value_of("author").unwrap();

    let repo = Repo::open(path)?;
    let diff = super::diff::diff(&repo, path)?;

    // TODO: this is not very efficient: we're reading the file twice.
    let mut f = File::open(path)?;
    let mut contents = Vec::new();
    f.read_to_end(&mut contents)?;
    let new_file = storage::File::from_bytes(&contents);
    if let Some(old_file) = repo.file("master") {
        let changes = Changes::from_diff(&old_file, &new_file, &diff);
        let patch = UnidentifiedPatch::new(author.to_owned(), msg.to_owned(), changes);

        // Write the patch to a temporary file, and get back the identified patch.
        let mut out = tempfile::NamedTempFile::new_in(".")
            .map_err(|e| (e, "trying to create a named temp file"))?;
        let patch = patch.write_out(&mut out)?;

        // Now that we know the patch's id, move it to a location given by that name.
        // TODO: more informative error
        out.persist(&patch.id.filename())
            .map_err(|e| (e.error, "trying to rename temp file"))?;
    } else {
        panic!("FIXME");
    };
    Ok(())
}

