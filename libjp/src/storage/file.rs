// Copyright 2018-2019 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//
// See the LICENSE-APACHE or LICENSE-MIT files at the top-level directory
// of this distribution.

use crate::storage::Storage;
use crate::NodeId;

/// A `File` is a special case of a [`Graggle`](crate::Graggle), in which there is just a linear order.
///
/// This struct offers convenient (read-only) access to a `File`, allowing the contents and ids of
/// nodes to be access by indexing.
///
/// The most convenient way to get a [`File`] is through [`Repo::file`](crate::Repo::file), but they can also
/// be built from raw bytes (using [`File::from_bytes`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    ids: Vec<NodeId>,
    // The contents of the file, in one long vector.
    contents: Vec<u8>,
    // The ith node is in contents[boundaries[i]..boundaries[i+1]]. In particular, boundaries is
    // always one longer than ids.
    boundaries: Vec<usize>,
}

impl File {
    /// Creates a `File` from a slice of node ids. The contents of those nodes will be retrieved
    /// from `storage`.
    pub(crate) fn from_ids(ids: &[NodeId], storage: &Storage) -> File {
        let mut contents = Vec::new();
        let mut boundaries = Vec::new();
        for id in ids {
            boundaries.push(contents.len());
            contents.extend_from_slice(storage.contents(id));
        }
        boundaries.push(contents.len());
        File {
            contents,
            boundaries,
            ids: ids.to_owned(),
        }
    }

    /// Creates a [`File`] from the raw bytes, by dividing them into lines.
    ///
    /// The [`NodeId`]s will be synthesized: they will have empty [`PatchId`](crate::PatchId)s, and
    /// their node indices will be consecutive, starting from zero.
    pub fn from_bytes(bytes: &[u8]) -> File {
        let contents = bytes.to_owned();

        // Finds the positions of the beginnings of all the lines, including the position of the
        // EOF if there isn't a newline at the end of the file.
        let mut boundaries = vec![0];
        boundaries.extend(
            bytes
                .iter()
                .enumerate()
                .filter(|&(_, &b)| b == b'\n')
                .map(|x| x.0 + 1),
        );
        if let Some(&last) = bytes.last() {
            if last != b'\n' {
                boundaries.push(bytes.len());
            }
        }

        let ids = (0..(boundaries.len() as u64 - 1))
            .map(NodeId::cur)
            .collect();

        File {
            ids,
            contents,
            boundaries,
        }
    }

    /// How many nodes does this file have?
    ///
    /// Currently, "nodes" is synonymous with "lines", but that may not necessarily be the case in
    /// the future (for example, we could diff files based on words instead of lines).
    pub fn num_nodes(&self) -> usize {
        self.ids.len()
    }

    /// Gets the contents of the node at the given index. This includes the `\n` character, if
    /// there was one.
    pub fn node(&self, idx: usize) -> &[u8] {
        let start = self.boundaries[idx];
        let end = self.boundaries[idx + 1];
        &self.contents[start..end]
    }

    /// Gets the id of the node at the given index.
    pub fn node_id(&self, idx: usize) -> &NodeId {
        &self.ids[idx]
    }

    /// Gets the whole file, as an array of bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.contents[..]
    }
}

#[cfg(test)]
mod tests {
    use super::File;

    #[test]
    fn from_bytes_empty() {
        let f = File::from_bytes(b"");
        assert_eq!(f.boundaries, vec![0]);
        assert_eq!(f.num_nodes(), 0);
        assert_eq!(f.ids.len(), 0);
    }

    #[test]
    fn from_bytes_one_empty_line() {
        let f = File::from_bytes(b"\n");
        assert_eq!(f.boundaries, vec![0, 1]);
        assert_eq!(f.num_nodes(), 1);
        assert_eq!(f.ids.len(), 1);
    }

    #[test]
    fn from_bytes_one_line_no_newline() {
        let f = File::from_bytes(b"test");
        assert_eq!(f.boundaries, vec![0, 4]);
        assert_eq!(f.num_nodes(), 1);
        assert_eq!(f.ids.len(), 1);
        assert_eq!(f.node(0), b"test");
    }

    #[test]
    fn from_bytes_one_line() {
        let f = File::from_bytes(b"test\n");
        assert_eq!(f.boundaries, vec![0, 5]);
        assert_eq!(f.num_nodes(), 1);
        assert_eq!(f.ids.len(), 1);
        assert_eq!(f.node(0), b"test\n");
    }

    #[test]
    fn from_bytes_two_lines() {
        let f = File::from_bytes(b"test1\ntest2\n");
        assert_eq!(f.boundaries, vec![0, 6, 12]);
        assert_eq!(f.num_nodes(), 2);
        assert_eq!(f.ids.len(), 2);
        assert_eq!(f.node(0), b"test1\n");
        assert_eq!(f.node(1), b"test2\n");
    }
}
