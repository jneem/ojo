use clap::ArgMatches;
use failure::Error;
use libjp::PatchId;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let hash = m.value_of("PATCH").unwrap();

    let repo = crate::open_repo()?;
    let id = PatchId::from_base64(hash)?;
    let patch = repo.open_patch(&id)?;
    let out_file = std::fs::File::create(hash)?;

    patch.into_unidentified().write_out(out_file)?;

    eprintln!("Successfully wrote the file '{}'", hash);
    Ok(())
}
