use clap::ArgMatches;
use failure::{Error, ResultExt};
use libojo::PatchId;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let hash = m.value_of("PATCH").unwrap();
    let out = m.value_of("output").unwrap_or(hash);

    let repo = crate::open_repo()?;
    let id = PatchId::from_base64(hash)?;
    let patch_data = repo.open_patch_data(&id)?;
    std::fs::write(out, patch_data).with_context(|_| format!("Couldn't create file '{}'", out))?;

    eprintln!("Successfully wrote the file '{}'", out);
    Ok(())
}
