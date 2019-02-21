use clap::ArgMatches;
use failure::{err_msg, Error, ResultExt};
use libquilt::{Change, Changes, NodeId, Repo};
use std::io::{stdin, Read};

fn parse_edge(s: &str) -> Option<(usize, usize)> {
    let dash_idx = s.find('-')?;
    let u: usize = s[..dash_idx].trim().parse().ok()?;
    let v: usize = s[(dash_idx + 1)..].trim().parse().ok()?;
    Some((u, v))
}

pub fn run(_m: &ArgMatches<'_>) -> Result<(), Error> {
    let dir = std::env::current_dir().context("Couldn't open the current directory.")?;
    let mut repo = Repo::init(&dir)?;
    // We need to write the repo before creating the patch, so that the directories all exist.
    repo.write()
        .context("Failed to write repository to disk.")?;

    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf)?;
    let buf = String::from_utf8(buf).context("Expected stdin to be UTF-8, but it wasn't.")?;
    let edges = buf
        .split_whitespace()
        .map(|s| parse_edge(s).ok_or_else(|| format_err!("Failed to parse '{}'.", s)))
        .collect::<Result<Vec<_>, _>>()?;

    let max_node = edges
        .iter()
        .map(|&(x, y)| x.max(y))
        .max()
        .ok_or_else(|| err_msg("Input was empty."))?;
    let new_nodes = (0..=max_node).map(|i| Change::NewNode {
        id: NodeId::cur(i as u64),
        contents: format!("Line {}\n", i).into_bytes(),
    });
    let new_edges = edges.into_iter().map(|(i, j)| Change::NewEdge {
        src: NodeId::cur(i as u64),
        dest: NodeId::cur(j as u64),
    });
    let changes = Changes {
        changes: new_nodes.chain(new_edges).collect::<Vec<_>>(),
    };
    let id = repo.create_patch("Anonymous bot", "Synthesized", changes)?;
    repo.apply_patch("master", &id)?;
    repo.write()
        .context("Failed to write repository to disk.")?;

    eprintln!("Synthesized a quilt repository.");
    Ok(())
}
