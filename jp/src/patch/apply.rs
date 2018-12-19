use clap::ArgMatches;
use failure::Error;
use libjp::PatchId;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let patch_id = m.value_of("PATCH").unwrap();
    let patch_id = PatchId::from_filename(patch_id)?;

    let mut repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);

    if m.is_present("revert") {
        let unapplied = repo.unapply_patch(&branch, &patch_id)?;
        if unapplied.is_empty() {
            eprintln!("No patches to unapply.");
        } else {
            eprintln!("Unapplied:");
            for u in unapplied {
                eprintln!("  {}", u.filename());
            }
        }
    } else {
        let applied = repo.apply_patch(&branch, &patch_id)?;
        if applied.is_empty() {
            eprintln!("No patches to apply.");
        } else {
            eprintln!("Applied:");
            for a in applied {
                eprintln!("  {}", a.filename());
            }
        }
    }

    repo.write()?;
    Ok(())
}
