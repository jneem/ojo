use clap::ArgMatches;
use failure::Error;
use graph::Graph;
use libjp::LineId;
use std::fs::File;
use std::io::prelude::*;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let output = m.value_of("out").unwrap_or("out.dot");
    let repo = super::open_repo()?;
    let inode = repo.storage().inode("master").unwrap();
    let digle = repo.storage().digle(inode);

    let mut output = File::create(output)?;
    let node_id = |n: &LineId| format!("{}:{}", &n.patch.filename()[1..8], n.line);
    writeln!(output, "digraph {{")?;
    for node in digle.nodes() {
        let id = node_id(&node);

        let mut label = String::from_utf8_lossy(repo.storage().contents(&node)).to_string();
        label.push('\n');
        label.push_str(&id);

        let style = if digle.is_live(&node) {
            "solid"
        } else {
            "dashed"
        };
        writeln!(
            output,
            "\"{}\" [style={},label={:?}];",
            node_id(&node),
            style,
            label
        )?;
        for nbr in digle.out_neighbors(&node) {
            let nbr_id = node_id(&nbr);
            writeln!(output, "\"{}\" -> \"{}\";", id, nbr_id)?;
        }
    }
    writeln!(output, "}}")?;

    Ok(())
}
