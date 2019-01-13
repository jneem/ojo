use askama_escape::escape;
use clap::ArgMatches;
use failure::Error;
use graph::Graph;
use libjp::decomposed_digle;
use libjp::{NodeId, Repo};
use std::fs::File;
use std::io::prelude::*;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let output = m.value_of("out").unwrap_or("out.dot");
    let repo = super::open_repo()?;
    let digle = repo.digle("master")?;
    let digle_decomp = decomposed_digle::Digle::from_digle(digle);

    let mut output = File::create(output)?;
    writeln!(output, "digraph {{")?;
    for idx in digle_decomp.nodes() {
        match digle_decomp.node_contents(idx) {
            decomposed_digle::Node::Single(id) => {
                write_single_node(&mut output, &repo, digle, &id, idx)?;
            }
            decomposed_digle::Node::Chain(ref ids) => {
                write_chain_node(&mut output, &repo, digle, &*ids, idx)?;
            }
        }

        for nbr_idx in digle_decomp.out_neighbors(&idx) {
            writeln!(output, "\"{}\" -> \"{}\";", idx, nbr_idx)?;
        }
    }
    writeln!(output, "}}")?;

    Ok(())
}

fn node_id(n: &NodeId) -> String {
    format!("{}/{:04}", escape(&n.patch.to_base64()[0..4]), n.node)
}

fn single_node_label(repo: &Repo, digle: libjp::Digle, id: &NodeId) -> String {
    let contents = String::from_utf8_lossy(repo.contents(&id)).to_string();

    if digle.is_live(id) {
        format!(
            "<font color=\"gray\">{}:</font> {}",
            node_id(id),
            escape(contents.trim_end())
        )
    } else {
        format!(
            "<s><font color=\"gray\">{}:</font> {}</s>",
            node_id(id),
            escape(contents.trim_end())
        )
    }
}

fn write_single_node<W: std::io::Write>(
    mut write: W,
    repo: &Repo,
    digle: libjp::Digle,
    id: &NodeId,
    idx: usize,
) -> Result<(), Error> {
    writeln!(
        write,
        "\"{}\" [shape=box, style=rounded, label=<{}>]",
        idx,
        single_node_label(repo, digle, id)
    )?;
    Ok(())
}

fn write_chain_node<W: std::io::Write>(
    mut write: W,
    repo: &Repo,
    digle: libjp::Digle,
    ids: &[NodeId],
    idx: usize,
) -> Result<(), Error> {
    let mut label = ids
        .iter()
        .map(|id| single_node_label(repo, digle, id))
        .collect::<Vec<String>>()
        .join("<br align=\"left\"/>");
    // Graphviz defaults to centering the text. To left-align it all, we put <br align="left"/> at
    // the end of every line (including the last one).
    label.push_str("<br align=\"left\"/>");

    writeln!(
        write,
        "\"{}\" [shape=box, style=rounded, label=<{}>]",
        idx, label
    )?;
    Ok(())
}
