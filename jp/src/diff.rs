use clap::ArgMatches;
use colored::*;
use diff::LineDiff;
use failure::Error;
use libjp::Repo;
use std::fmt;
use std::io::prelude::*;

pub struct Diff {
    pub changes: Vec<LineDiff>,
    pub a_lines: Vec<Vec<u8>>,
    pub b_lines: Vec<Vec<u8>>,
}

impl fmt::Display for Diff {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &ch in &self.changes {
            match ch {
                LineDiff::New(i) => {
                    let s = format!("+ {}", String::from_utf8_lossy(&self.b_lines[i]));
                    write!(fmt, "{}", s.green())?;
                }
                LineDiff::Delete(i) => {
                    let s = format!("- {}", String::from_utf8_lossy(&self.a_lines[i]));
                    write!(fmt, "{}", s.red())?;
                }
                LineDiff::Keep(i, _) => {
                    write!(fmt, "  {}", String::from_utf8_lossy(&self.a_lines[i]))?;
                }
            }
        }
        Ok(())
    }
}

// TODO: we should refactor some of this into libjp. In particular, it's probably useful to
// have a method for taking a file and producing a diff.
pub fn diff(repo: &Repo) -> Result<Diff, Error> {
    let mut fs_file = repo.open_file()?;
    let mut fs_file_contents = Vec::new();
    fs_file.read_to_end(&mut fs_file_contents)?;
    let fs_lines = lines(&fs_file_contents);

    if let Some(repo_file) = repo.file("master") {
        let repo_lines = (0..repo_file.num_lines())
            .map(|i| repo_file.line(i).to_owned())
            .collect::<Vec<_>>();
        let line_diffs = diff::diff(&repo_lines, &fs_lines);
        let ret = Diff {
            changes: line_diffs,
            a_lines: repo_lines,
            b_lines: fs_lines,
        };
        Ok(ret)
    } else {
        panic!("FIXME");
    }
}

pub fn run(_m: &ArgMatches<'_>) -> Result<(), Error> {
    let repo = super::open_repo()?;

    let diff = diff(&repo)?;
    print!("{}", diff);

    Ok(())
}

// Splits a file into \n-separated lines. This differs from the method in the standard library in
// that it keeps the line endings.
// TODO: we should (after benchmarking) revisit how we're comparing files. For example, it might be
// worth interning things for quicker comparisons, and possibly reduced allocations.
fn lines(input: &[u8]) -> Vec<Vec<u8>> {
    let mut ret = Vec::new();
    let mut cur_idx = 0;
    for (newline_idx, _) in input.into_iter().enumerate().filter(|&(_, &b)| b == b'\n') {
        ret.push(input[cur_idx..=newline_idx].to_owned());
        cur_idx = newline_idx + 1;
    }
    if input.is_empty() || input.last().unwrap() != &b'\n' {
        ret.push(input[cur_idx..].to_owned());
    }
    ret
}

#[cfg(test)]
mod tests {
    #[test]
    fn lines() {
        assert_eq!(super::lines(&b""[..]), vec![Vec::<u8>::new()]);
        assert_eq!(super::lines(&b"\n"[..]), vec![b"\n".to_owned()]);
        assert_eq!(
            super::lines(&b"a\nb\n"[..]),
            vec![b"a\n".to_owned(), b"b\n".to_owned()]
        );
        assert_eq!(
            super::lines(&b"a\nb"[..]),
            vec![b"a\n"[..].to_owned(), b"b"[..].to_owned()]
        );
    }
}
