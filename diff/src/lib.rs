#[cfg(test)]
#[macro_use]
extern crate proptest;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

mod lis;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LineDiff {
    /// This line was introduced in the second file, and the `usize` is the line number in the
    /// second file.
    New(usize),
    /// This line was in the first file, but got deleted.
    Delete(usize),
    /// This line was present in both files; the first `usize` is the line number in the first
    /// file, and the second is the line number in the second file.
    Keep(usize, usize),
}

// This is a little trick for associating an element with its line number in a file. The point is
// that our implementation of Hash and Eq will ignore the index, so we can put `WithIndex` in
// hash maps and the index will just be transparently carried along.
#[derive(Eq, PartialOrd)]
struct WithIndex<T> {
    idx: usize,
    elem: T,
}

impl<T: PartialEq> PartialEq for WithIndex<T> {
    fn eq(&self, other: &WithIndex<T>) -> bool {
        self.elem.eq(&other.elem)
    }
}

impl<T: Hash> Hash for WithIndex<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.elem.hash(state);
    }
}

// Returns a map pointing from lines to the number of times they appeared in the file. Each line is
// also associated with the index where it first appeared.
fn line_counts<T: Hash + Eq>(lines: &[T]) -> HashMap<WithIndex<&T>, usize> {
    let mut line_counts = HashMap::new();

    for (line_idx, line) in lines.iter().enumerate() {
        let elem = WithIndex {
            elem: line,
            idx: line_idx,
        };
        *line_counts.entry(elem).or_insert(0) += 1;
    }
    line_counts
}

// The returned value `ret` is the largest number such that the first `ret` elements of `a` are the
// same as the first `ret` elements of `b`.
fn prefix_len<T: Eq>(a: &[T], b: &[T]) -> usize {
    a.into_iter()
        .zip(b.into_iter())
        .take_while(|(x, y)| x == y)
        .count()
}

// The returned value `ret` is the largest number such that the last `ret` elements of `a` are the
// same as the last `ret` elements of `b`.
fn suffix_len<T: Eq>(a: &[T], b: &[T]) -> usize {
    a.into_iter()
        .rev()
        .zip(b.into_iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

// Returns the tuple (pref_len, a_mid, b_mid, suff_len), where `pref_len` is the length of the
// common prefix of `a` and `b`, `suff_len` is the length of the common suffix of `a` and `b`, and
// `a_mid` and `b_mid` are the parts of `a` and `b` that are left after the prefixes and suffixes
// are removed.
fn match_ends<'a, T: Eq>(a: &'a [T], b: &'a [T]) -> (usize, &'a [T], &'a [T], usize) {
    let pref_len = prefix_len(a, b);
    let suff_len = suffix_len(&a[pref_len..], &b[pref_len..]);
    let a_mid = &a[pref_len..(a.len() - suff_len)];
    let b_mid = &b[pref_len..(b.len() - suff_len)];
    (pref_len, a_mid, b_mid, suff_len)
}

// This function calculates an extremely simple kind of diff: find the longest common prefix and
// suffix of the two files. Everything that isn't part of the prefix and suffix gets marked as
// changed.
//
// We also support adding an offset to the line numbers of the two files, since they might actually
// refer just to smaller parts of larger files.
fn diff_ends<T: Eq>(a: &[T], a_offset: usize, b: &[T], b_offset: usize, diff: &mut Vec<LineDiff>) {
    let (pref_len, a_mid, b_mid, suff_len) = match_ends(a, b);
    for i in 0..pref_len {
        diff.push(LineDiff::Keep(a_offset + i, b_offset + i));
    }
    for i in 0..a_mid.len() {
        diff.push(LineDiff::Delete(a_offset + pref_len + i));
    }
    for i in 0..b_mid.len() {
        diff.push(LineDiff::New(b_offset + pref_len + i));
    }
    for i in 0..suff_len {
        diff.push(LineDiff::Keep(
            a_offset + pref_len + a_mid.len() + i,
            b_offset + pref_len + b_mid.len() + i,
        ));
    }
}

pub fn diff<T: Hash + Eq>(a: &[T], b: &[T]) -> Vec<LineDiff> {
    let (pref_len, a_mid, b_mid, suff_len) = match_ends(a, b);
    let a_line_counts = line_counts(a_mid);
    let mut b_line_counts = line_counts(b_mid);
    let a_unique = a_line_counts
        .into_iter()
        .filter(|(_, count)| *count == 1)
        .map(|(line, _)| line);

    // `both_unique` is a Vec of (usize, usize) pairs corresponding to lines that are unique in
    // both files. The first usize is the index *in file b* and the second is the index in file a,
    // and `both_unique` will be sorted according to the index in file a. The order of the indices
    // seems backwards, but the point is that we'll look for a longest increasing subsequence and
    // we want "increasing" here to mean according to appearance in file b.
    let mut both_unique = a_unique
        .filter_map(|a_line| {
            // TODO: This is a bit awkward, but it can get better if HashMap::get_key_value is
            // stabilized.
            let a_idx = a_line.idx;
            if let Entry::Occupied(entry) = b_line_counts.entry(a_line) {
                if entry.get() == &1 {
                    return Some((entry.key().idx, a_idx));
                }
            }
            None
        })
        .collect::<Vec<(usize, usize)>>();
    both_unique.sort_unstable_by_key(|(_b_idx, a_idx)| *a_idx);

    let mut ret = Vec::with_capacity(a.len().max(b.len()));
    for i in 0..pref_len {
        ret.push(LineDiff::Keep(i, i));
    }

    let lis = lis::longest_increasing_subsequence(&both_unique);
    let mut prev_b_idx = 0;
    let mut prev_a_idx = 0;
    for i in lis {
        let (next_b_idx, next_a_idx) = both_unique[i];
        let a_chunk = &a_mid[prev_a_idx..next_a_idx];
        let b_chunk = &b_mid[prev_b_idx..next_b_idx];
        diff_ends(
            a_chunk,
            pref_len + prev_a_idx,
            b_chunk,
            pref_len + prev_b_idx,
            &mut ret,
        );
        prev_b_idx = next_b_idx;
        prev_a_idx = next_a_idx;
    }

    let a_chunk = &a_mid[prev_a_idx..];
    let b_chunk = &b_mid[prev_b_idx..];
    diff_ends(
        a_chunk,
        pref_len + prev_a_idx,
        b_chunk,
        pref_len + prev_b_idx,
        &mut ret,
    );

    for i in 0..suff_len {
        ret.push(LineDiff::Keep(
            a.len() - suff_len + i,
            b.len() - suff_len + i,
        ));
    }

    ret
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use std::fmt::Debug;

    use super::LineDiff::*;
    use super::*;

    macro_rules! test_diff_ends {
        ($name:ident, $a:expr, $b:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let a: &[_] = &$a[..];
                let b: &[_] = &$b[..];
                let expected: &[_] = &$expected[..];
                let mut diff = Vec::new();
                diff_ends(a, 0, b, 0, &mut diff);
                assert_eq!(diff.as_slice(), expected);
            }
        };
    }

    test_diff_ends!(
        diff_ends_all,
        [1, 2, 3],
        [1, 2, 3],
        [Keep(0, 0), Keep(1, 1), Keep(2, 2),]
    );
    test_diff_ends!(
        diff_ends_shorter_first,
        [1, 1],
        [1, 1, 1],
        [Keep(0, 0), Keep(1, 1), New(2),]
    );
    test_diff_ends!(
        diff_ends_longer_first,
        [1, 1, 1],
        [1, 1],
        [Keep(0, 0), Keep(1, 1), Delete(2),]
    );

    // A diff between two files is valid if and only if
    // - every input index appears exactly once in the diff, in increasing order
    // - every output index appears exactly once in the diff, in increasing order
    // - for every Keep line in the diff, the input and output lines are the same.
    fn assert_valid<T: Debug + Eq>(a: &[T], b: &[T], diff: &[LineDiff]) {
        let input_indices = diff
            .iter()
            .filter_map(|line| match *line {
                New(_) => None,
                Keep(i, _) => Some(i),
                Delete(i) => Some(i),
            })
            .collect::<Vec<_>>();
        assert_eq!(input_indices, (0..a.len()).into_iter().collect::<Vec<_>>());

        let output_indices = diff
            .iter()
            .filter_map(|line| match *line {
                New(i) => Some(i),
                Keep(_, i) => Some(i),
                Delete(_) => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(output_indices, (0..b.len()).into_iter().collect::<Vec<_>>());

        for line in diff {
            if let &Keep(i, j) = line {
                assert_eq!(a[i], b[j]);
            }
        }
    }

    // We generate files by mostly generating common numbers (up to 10), and occasionally
    // rare numbers (up to 1000). The common numbers are to make diff's job harder, and the rare
    // numbers are to ensure that there are some unique lines.
    fn file() -> BoxedStrategy<Vec<i32>> {
        prop::collection::vec(
            prop::strategy::Union::new_weighted(vec![(10, 0..10), (1, 0..1000)]),
            1..100,
        )
        .boxed()
    }

    // Generates two files for diffing by first generating one, and then making another by changing
    // the first one a bit.
    fn two_files() -> BoxedStrategy<(Vec<i32>, Vec<i32>)> {
        file()
            .prop_perturb(|f, mut rng| {
                let mut g = f.clone();
                // Make between 0 and 19 random changes.
                for _ in 0..rng.gen_range(0, 20) {
                    let g_len = g.len();
                    match rng.choose(&[1, 2, 3]).unwrap() {
                        1 => {
                            // delete a line
                            if !g.is_empty() {
                                g.remove(rng.gen_range(0, g_len));
                            }
                        }
                        2 => {
                            // insert a line
                            g.insert(rng.gen_range(0, g_len + 1), rng.gen_range(0, 10));
                        }
                        3 => {
                            // swap two lines
                            if !g.is_empty() {
                                g.swap(rng.gen_range(0, g_len), rng.gen_range(0, g_len));
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                (f, g)
            })
            .boxed()
    }

    proptest! {
        #[test]
        fn test_valid_diff((f, g) in two_files()) {
            let d = diff(&f, &g);
            assert_valid(&f, &g, &d);
        }
    }
}
