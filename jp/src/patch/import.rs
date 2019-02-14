use clap::ArgMatches;
use failure::{Error, ResultExt};

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let path = m.value_of("PATH").unwrap();

    let mut repo = crate::open_repo()?;
    let contents =
        std::fs::read(path).with_context(|_| format!("Failed to read file '{}'", path))?;
    let id = repo.register_patch(&contents)?;
    repo.write()?;

    eprintln!("Successfully imported a patch with id {}", id.to_base64());
    Ok(())
}
