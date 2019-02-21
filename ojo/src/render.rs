use clap::ArgMatches;
use failure::{err_msg, Error};

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let path = crate::file_path(m);
    let repo = crate::open_repo()?;
    let branch = crate::branch(&repo, m);
    let file = repo.file(&branch).map_err(|e| match e {
        libojo::Error::NotOrdered => {
            err_msg("Couldn't render a file, because the data isn't ordered")
        }
        other => other.into(),
    })?;

    std::fs::write(&path, file.as_bytes())?;
    eprintln!("Successfully wrote file '{}'", path);

    Ok(())
}
