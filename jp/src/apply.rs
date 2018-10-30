use clap::ArgMatches;
use failure::Error;
use libjp::PatchId;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let patch_id = m.value_of("PATCH").unwrap();
    let patch_id = PatchId::from_filename(patch_id)?;

    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, m);

    if m.is_present("revert") {
        repo.unapply_patch(&branch, &patch_id)?;
    } else {
        repo.apply_patch(&branch, &patch_id)?;
    }

    // The unwrap is ok, because if the branch didn't exist then the previous line would already
    // have failed.
    repo.digle(&branch).unwrap().assert_consistent();

    repo.write()?;
    Ok(())
}

