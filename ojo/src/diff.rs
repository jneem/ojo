use {
    anyhow::{Context, Result, anyhow},
    clap::ArgMatches,
    colored::*,
    libojo::Repo,
    ojo_diff::LineDiff,
    std::fmt,
};

pub struct DiffDisplay(pub libojo::Diff);

impl fmt::Display for DiffDisplay {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &ch in &self.0.diff {
            match ch {
                LineDiff::New(i) => {
                    let s = format!("+ {}", String::from_utf8_lossy(self.0.file_b.node(i)));
                    write!(fmt, "{}", s.green())?;
                }
                LineDiff::Delete(i) => {
                    let s = format!("- {}", String::from_utf8_lossy(self.0.file_a.node(i)));
                    write!(fmt, "{}", s.red())?;
                }
                LineDiff::Keep(i, _) => {
                    write!(fmt, "  {}", String::from_utf8_lossy(self.0.file_a.node(i)))?;
                }
            }
        }
        Ok(())
    }
}

pub fn diff(repo: &Repo, branch: &str, file_name: &str) -> Result<libojo::Diff> {
    let mut path = repo.root_dir.clone();
    path.push(file_name);
    let fs_file_contents =
        std::fs::read(&path).with_context(|| format!("Reading the file {}", file_name))?;

    repo.diff(branch, &fs_file_contents[..]).map_err(|e| {
        if let libojo::Error::NotOrdered = e {
            anyhow!("Cannot create a diff because the repo's contents aren't ordered")
        } else {
            e.into()
        }
    })
}

pub fn run(m: &ArgMatches<'_>) -> Result<()> {
    let repo = super::open_repo()?;
    let branch = super::branch(&repo, m);
    let file_name = super::file_path(m);

    let diff = diff(&repo, &branch, &file_name)?;
    print!("{}", DiffDisplay(diff));

    Ok(())
}
