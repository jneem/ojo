use crate::LineId;
use crate::storage::Storage;

/// A File is a special case of a Digle, in which there is just a linear order.
///
/// This struct offers convenient (read-only) access to a File, allowing the contents and ids of
/// lines to be access by indexing.
#[derive(Clone, Debug)]
pub struct File {
    ids: Vec<LineId>,
    // The contents of the file, in one long vector.
    contents: Vec<u8>,
    // The ith line is in contents[boundaries[i]..boundaries[i+1]]. In particular, boundaries is
    // always one longer than ids.
    boundaries: Vec<usize>,
}

impl File {
    /// Creates a `File` from a slice of line ids. The contents of those lines will be retrieved
    /// from `storage`.
    pub fn from_ids(ids: &[LineId], storage: &Storage) -> File {
        let mut contents = Vec::new();
        let mut boundaries = Vec::new();
        for id in ids {
            boundaries.push(contents.len());
            contents.extend_from_slice(storage.contents(id));
        }
        boundaries.push(contents.len());
        File {
            contents: contents,
            boundaries: boundaries,
            ids: ids.to_owned(),
        }
    }

    /// Creates a `File` from the raw bytes, by dividing them into lines.
    ///
    /// The `LineId`s will be synthesized: they will have empty `PatchId`s, and their line indices
    /// will be consecutive, starting from zero.
    pub fn from_bytes(bytes: &[u8]) -> File {
        let contents = bytes.to_owned();

        // Finds the positions of the beginnings of all the lines, including the position of the
        // EOF if there isn't a newline at the end of the file.
        let mut boundaries = vec![0];
        boundaries.extend(
            bytes.into_iter()
            .enumerate()
            .filter(|&(_, &b)| b == b'\n')
            .map(|x| x.0 + 1)
        );
        if let Some(&last) = bytes.last() {
            if last != b'\n' {
                boundaries.push(bytes.len());
            }
        }

        let ids = (0..(boundaries.len() as u64 - 1)).map(LineId::cur).collect();

        File {
            ids,
            contents,
            boundaries,
        }
    }

    /// How many lines does this file have?
    pub fn num_lines(&self) -> usize {
        self.ids.len()
    }

    pub fn line(&self, idx: usize) -> &[u8] {
        let start = self.boundaries[idx];
        let end = self.boundaries[idx + 1];
        &self.contents[start..end]
    }

    pub fn line_id(&self, idx: usize) -> &LineId {
        &self.ids[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::File;

    #[test]
    fn from_bytes_empty() {
        let f = File::from_bytes(b"");
        assert_eq!(f.boundaries, vec![0]);
        assert_eq!(f.num_lines(), 0);
        assert_eq!(f.ids.len(), 0);
    }

    #[test]
    fn from_bytes_one_empty_line() {
        let f = File::from_bytes(b"\n");
        assert_eq!(f.boundaries, vec![0, 1]);
        assert_eq!(f.num_lines(), 1);
        assert_eq!(f.ids.len(), 1);
    }

    #[test]
    fn from_bytes_one_line_no_newline() {
        let f = File::from_bytes(b"test");
        assert_eq!(f.boundaries, vec![0, 4]);
        assert_eq!(f.num_lines(), 1);
        assert_eq!(f.ids.len(), 1);
        assert_eq!(f.line(0), b"test");
    }

    #[test]
    fn from_bytes_one_line() {
        let f = File::from_bytes(b"test\n");
        assert_eq!(f.boundaries, vec![0, 5]);
        assert_eq!(f.num_lines(), 1);
        assert_eq!(f.ids.len(), 1);
        assert_eq!(f.line(0), b"test\n");
    }

    #[test]
    fn from_bytes_two_lines() {
        let f = File::from_bytes(b"test1\ntest2\n");
        assert_eq!(f.boundaries, vec![0, 6, 12]);
        assert_eq!(f.num_lines(), 2);
        assert_eq!(f.ids.len(), 2);
        assert_eq!(f.line(0), b"test1\n");
        assert_eq!(f.line(1), b"test2\n");
    }
}

