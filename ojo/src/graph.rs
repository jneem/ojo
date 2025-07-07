use {
    anyhow::Result,
    askama_escape::{Html, escape},
    clap::Parser,
    libojo::{ChainGraggle, NodeId, Repo},
    ojo_graph::Graph,
    std::{fs::File, io::prelude::*, path::PathBuf},
};

#[derive(Parser, Debug)]
pub struct Opts {
    /// path for the output file
    #[arg(short, long, default_value = "out.dot")]
    out: PathBuf,
}

pub fn run(opts: Opts) -> Result<()> {
    let repo = super::open_repo()?;
    let graggle = repo.graggle("master")?;
    // TODO: allow retrieving only the live graph
    let graggle_decomp = ChainGraggle::from_graph(graggle.as_full_graph());

    let mut output = File::create(opts.out)?;

    writeln!(output, "digraph {{")?;
    for idx in graggle_decomp.nodes() {
        let chain = graggle_decomp.chain(idx);
        if chain.len() == 1 {
            write_single_node(&mut output, &repo, graggle, &chain[0], idx)?;
        } else {
            write_chain_node(&mut output, &repo, graggle, chain, idx)?;
        }

        for neighbor_idx in graggle_decomp.out_neighbors(&idx) {
            writeln!(output, "\"{idx}\" -> \"{neighbor_idx}\";")?;
        }
    }
    writeln!(output, "}}")?;

    Ok(())
}

fn node_id(n: &NodeId) -> String {
    format!("{}/{:04}", escape(&n.patch.to_base64()[0..4], Html), n.node)
}

fn single_node_label(repo: &Repo, graggle: libojo::Graggle, id: &NodeId) -> String {
    let contents = String::from_utf8_lossy(repo.contents(id)).to_string();
    let contents = escape(contents.trim_end(), Html);
    let node_id = node_id(id);

    if graggle.is_live(id) {
        format!("<font color=\"gray\">{node_id}:</font> {contents}")
    } else {
        format!("<s><font color=\"gray\">{node_id}:</font> {contents}</s>")
    }
}

fn write_single_node<W: std::io::Write>(
    mut write: W,
    repo: &Repo,
    graggle: libojo::Graggle,
    id: &NodeId,
    idx: usize,
) -> Result<()> {
    let label = single_node_label(repo, graggle, id);
    writeln!(
        write,
        "\"{idx}\" [shape=box, style=rounded, label=<{label}>]"
    )?;
    Ok(())
}

fn write_chain_node<W: std::io::Write>(
    mut write: W,
    repo: &Repo,
    graggle: libojo::Graggle,
    ids: &[NodeId],
    idx: usize,
) -> Result<()> {
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
        "\"{idx}\" [shape=box, style=rounded, label=<{label}>]"
    )?;
    Ok(())
}
