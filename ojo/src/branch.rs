use {
    anyhow::Result,
    clap::{Parser, Subcommand},
};

#[derive(Parser, Debug)]
pub struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Create a copy of the current branch
    Clone {
        /// name of the branch to create
        name: String,
    },
    /// Delete a branch
    Delete {
        /// name of the branch to delete
        name: String,
    },
    /// List all branches
    List,
    /// Create a new, empty, branch
    New {
        /// name of the branch to create
        name: String,
    },
    /// Switch the current branch
    Switch {
        /// name of the branch to switch to
        name: String,
    },
}

pub fn run(opts: Opts) -> Result<()> {
    match opts.subcmd {
        SubCommand::Clone { name } => clone_run(name),
        SubCommand::Delete { name } => delete_run(name),
        SubCommand::List => list_run(),
        SubCommand::New { name } => new_run(name),
        SubCommand::Switch { name } => switch_run(name),
    }
}

fn clone_run(name: String) -> Result<()> {
    let mut repo = crate::open_repo()?;
    let cur_branch = repo.current_branch.clone();
    repo.clone_branch(&cur_branch, &name)?;
    repo.write()?;
    println!("Cloned branch \"{cur_branch}\" to branch \"{name}\"");
    Ok(())
}

fn delete_run(name: String) -> Result<()> {
    let mut repo = crate::open_repo()?;
    repo.delete_branch(&name)?;
    repo.write()?;
    println!("Deleted branch \"{name}\"");
    Ok(())
}

fn list_run() -> Result<()> {
    let repo = crate::open_repo()?;
    let mut branches = repo.branches().collect::<Vec<_>>();
    branches.sort();
    for b in branches {
        if b == repo.current_branch {
            println!("* {b}");
        } else {
            println!("  {b}");
        }
    }
    Ok(())
}

fn new_run(name: String) -> Result<()> {
    let mut repo = crate::open_repo()?;
    repo.create_branch(&name)?;
    repo.write()?;
    println!("Created empty branch \"{name}\"");
    Ok(())
}

fn switch_run(name: String) -> Result<()> {
    let mut repo = crate::open_repo()?;
    repo.switch_branch(&name)?;
    repo.write()?;
    println!("Current branch is \"{name}\"");
    Ok(())
}
