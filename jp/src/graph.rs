use askama_escape::escape;
use clap::ArgMatches;
use failure::Error;
use graph::Graph;
use libjp::ChainGraggle;
use libjp::{NodeId, Repo};
use std::fs::File;
use std::io::prelude::*;

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    let output = m.value_of("out").unwrap_or("out.dot");
    let repo = super::open_repo()?;
    let graggle = repo.graggle("master")?;
    // TODO: allow retrieving only the live graph
    let graggle_decomp = ChainGraggle::from_graph(graggle.as_full_graph());

    let mut output = File::create(output)?;
    writeln!(output, "digraph {{")?;
    for idx in graggle_decomp.nodes() {
        let chain = graggle_decomp.chain(idx);
        if chain.len() == 1 {
            write_single_node(&mut output, &repo, graggle, &chain[0], idx)?;
        } else {
            write_chain_node(&mut output, &repo, graggle, chain, idx)?;
        }

        for nbr_idx in graggle_decomp.out_neighbors(&idx) {
            writeln!(output, "\"{}\" -> \"{}\";", idx, nbr_idx)?;
        }
    }
    writeln!(output, "}}")?;

    Ok(())
}

fn node_id(n: &NodeId) -> String {
    format!("{}/{:04}", escape(&n.patch.to_base64()[0..4]), n.node)
}

fn single_node_label(repo: &Repo, graggle: libjp::Graggle, id: &NodeId) -> String {
    let contents = String::from_utf8_lossy(repo.contents(&id)).to_string();

    if graggle.is_live(id) {
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
    graggle: libjp::Graggle,
    id: &NodeId,
    idx: usize,
) -> Result<(), Error> {
    writeln!(
        write,
        "\"{}\" [shape=box, style=rounded, label=<{}>]",
        idx,
        single_node_label(repo, graggle, id)
    )?;
    Ok(())
}

fn write_chain_node<W: std::io::Write>(
    mut write: W,
    repo: &Repo,
    graggle: libjp::Graggle,
    ids: &[NodeId],
    idx: usize,
) -> Result<(), Error> {
    let mut label = ids
        .iter()
        .map(|id| single_node_label(repo, graggle, id))
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
