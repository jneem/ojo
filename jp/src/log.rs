use clap::ArgMatches;
use failure::Error;
use libjp::Repo;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok because "path" is a required argument.
    let path = m.value_of("PATH").unwrap();
    let repo = Repo::open(path)?;

    for patch_id in repo.patches() {
        // FIXME: open the patch file and read the description. Also, this is wrong because we need
        // to somehow find out just the patches in the current branch.
        println!("{:?}", patch_id);
    }
    Ok(())
}

