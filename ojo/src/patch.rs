use {anyhow::Result, clap::ArgMatches};

mod apply;
pub mod create;
mod export;
mod import;

pub fn run(m: &ArgMatches<'_>) -> Result<()> {
    match m.subcommand_name() {
        Some("apply") => apply::run(m.subcommand_matches("apply").unwrap()),
        Some("create") => create::run(m.subcommand_matches("create").unwrap()),
        Some("export") => export::run(m.subcommand_matches("export").unwrap()),
        Some("import") => import::run(m.subcommand_matches("import").unwrap()),
        _ => panic!("Unknown subcommand"),
    }
}
