use clap::ArgMatches;
use failure::Error;

pub fn run(m: &ArgMatches) -> Result<(), Error> {
    match m.subcommand_name() {
        Some("clone") => clone_run(m.subcommand_matches("clone").unwrap()),
        Some("delete") => delete_run(m.subcommand_matches("delete").unwrap()),
        Some("list") => list_run(m.subcommand_matches("list").unwrap()),
        Some("new") => new_run(m.subcommand_matches("new").unwrap()),
        Some("switch") => switch_run(m.subcommand_matches("switch").unwrap()),
        _ => panic!("Unknown subcommand"),
    }
}

fn clone_run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok, because NAME is a required argument.
    let name = m.value_of("NAME").unwrap();
    let mut repo = crate::open_repo()?;
    let cur_branch = repo.current_branch.clone();
    repo.clone_branch(&cur_branch, name)?;
    repo.write()?;
    eprintln!("Cloned branch \"{}\" to branch \"{}\"", cur_branch, name);
    Ok(())
}

fn delete_run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok, because NAME is a required argument.
    let name = m.value_of("NAME").unwrap();
    let mut repo = crate::open_repo()?;
    repo.delete_branch(name)?;
    repo.write()?;
    eprintln!("Deleted branch \"{}\"", name);
    Ok(())
}

fn list_run(_m: &ArgMatches) -> Result<(), Error> {
    let repo = crate::open_repo()?;
    let mut branches = repo.storage().branches().collect::<Vec<_>>();
    branches.sort();
    for b in branches {
        if b == &repo.current_branch {
            println!("* {}", b);
        } else {
            println!("  {}", b);
        }
    }
    Ok(())
}

fn new_run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok, because NAME is a required argument.
    let name = m.value_of("NAME").unwrap();
    let mut repo = crate::open_repo()?;
    repo.create_branch(name)?;
    repo.write()?;
    eprintln!("Created empty branch \"{}\"", name);
    Ok(())
}

fn switch_run(m: &ArgMatches) -> Result<(), Error> {
    // The unwrap is ok, because NAME is a required argument.
    let name = m.value_of("NAME").unwrap();
    let mut repo = crate::open_repo()?;
    repo.switch_branch(name)?;
    repo.write()?;
    eprintln!("Current branch is \"{}\"", name);
    Ok(())
}
