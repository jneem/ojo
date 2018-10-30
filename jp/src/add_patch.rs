use clap::ArgMatches;
use failure::Error;
use libjp::patch::Patch;
use libjp::PatchId;
use std::fs::File;
use std::path::Path;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok because PATCH is a required argument.
    let patch_path = Path::new(m.value_of("PATCH").unwrap());

    let patch_filename = patch_path
        .file_name()
        .ok_or_else(|| format_err!("PATCH must be a path to a file"))?;
    // The patch filename is supposed to be the base64-encoding of the patch hash, so it can be
    // converted to &str.
    let patch_filename = patch_filename.to_str().ok_or_else(|| {
        format_err!(
            "PATCH name must be in base64 encoding, got: {:?}",
            patch_filename
        )
    })?;
    let patch_id = PatchId::from_filename(patch_filename)?;
    let patch = File::open(patch_path)?;
    let patch = Patch::from_reader(patch, patch_id)?;

    let mut repo = super::open_repo()?;
    repo.register_patch(&patch)?;
    Ok(())
}
