use clap::ArgMatches;
use libjp::{Patch, PatchId, Repo};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

pub fn run(m: &ArgMatches) -> Result<(), libjp::Error> {
    // The unwraps are ok because these are required arguments.
    let repo_path = m.value_of("PATH").unwrap();
    let patch_path = Path::new(m.value_of("PATCH").unwrap());

    // The filename of the patch is the base64 encoding of the patch id.
    let patch_filename = patch_path
        .file_name()
        .ok_or_else(|| libjp::Error::NoFilename(patch_path.to_owned()))?;
    // The patch filename is supposed to be base64 encoded, so it can be converted to &str.
    // However, we should have a better error message if it isn't (TODO)
    let patch_filename = patch_filename.to_str().unwrap();
    let patch_id = PatchId::from_filename(patch_filename)?;

    let mut repo = Repo::open(repo_path)?;
    let patch = File::open(patch_path)?;
    let patch = Patch::from_reader(patch, patch_id)?;

    // Copy the patch to the patch directory. FIXME: check for errors
    let mut target_path = repo.patch_dir.clone();
    target_path.push(patch_path);
    std::fs::copy(patch_path, &target_path)?;

    let inode = repo.storage().inode("master").unwrap(); // FIXME: unwrap and hardcoded name
    let mut digle = repo.storage().digle(inode); // FIXME: unwrap
    patch.store_new_contents(repo.storage_mut());
    patch.apply_to_digle(&mut digle);
    repo.storage_mut().set_digle(inode, digle);

    repo.write()
}
