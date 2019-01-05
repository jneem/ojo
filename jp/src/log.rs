use clap::ArgMatches;
use failure::Error;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let repo = super::open_repo()?;
    let branch = super::branch(&repo, m);

    for patch_id in repo.patches(&branch) {
        let patch = repo.open_patch(&patch_id)?;
        println!("patch {}", patch_id.to_base64());
        println!("Author: {}", patch.header().author);
        println!("");
        // TODO: dates and sorting.
        // TODO: better display for multi-line description.
        println!("\t{}", patch.header().description);
        println!("");
    }
    Ok(())
}
