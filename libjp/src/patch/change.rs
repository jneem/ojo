use diff::LineDiff;

use crate::{LineId, PatchId};
use crate::storage::{Digle, File, Storage};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Changes {
    pub changes: Vec<Change>,
}

// While iterating through the diff, we need to remember what the previous line was and where it
// came from: either there wasn't one, or it came from one of the two files.
enum LastLine<'a> {
    Start,
    File1(&'a LineId),
    File2(&'a LineId),
}

impl<'a> LastLine<'a> {
    fn either(&self) -> Option<&LineId> {
        match *self {
            LastLine::File1(i) => Some(i),
            LastLine::File2(i) => Some(i),
            LastLine::Start => None,
        }
    }
}

impl Changes {
    pub fn from_diff(file1: &File, file2: &File, diff: &[LineDiff]) -> Changes {
        let mut changes = Vec::new();
        let mut last = LastLine::Start;
        for d in diff {
            match *d {
                LineDiff::New(i) => { 
                    let id = file2.line_id(i);
                    changes.push(Change::NewNode {
                        id: id.clone(),
                        contents: file2.line(i).to_owned(),
                    });

                    // We are adding a new line, so we need to connect it to whatever line came
                    // before it, no matter where it came from.
                    if let Some(last_id) = last.either() {
                        changes.push(Change::NewEdge {
                            src: last_id.clone(),
                            dst: id.clone(),
                        });
                    }
                    last = LastLine::File2(id);
                }
                LineDiff::Keep(i, _) => {
                    let id = file1.line_id(i);

                    // If the last line came from the new file, we need to hook it up to this line.
                    if let LastLine::File2(last_id) = last {
                        changes.push(Change::NewEdge {
                            src: last_id.clone(),
                            dst: id.clone(),
                        });
                    }
                    last = LastLine::File1(id);
                }
                LineDiff::Delete(i) => {
                    let id = file1.line_id(i);
                    changes.push(Change::DeleteNode {
                        id: id.clone()
                    });
                }
            }
        }
        Changes { changes }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Change {
    NewNode { id: LineId, contents: Vec<u8> },
    DeleteNode { id: LineId },
    NewEdge { src: LineId, dst: LineId },
}

impl Change {
    pub fn apply_to_digle(&self, digle: &mut Digle) {
        match *self {
            Change::NewNode { ref id, .. } => digle.add_node(id.clone()),
            Change::DeleteNode { ref id } => digle.delete_node(&id),
            Change::NewEdge { ref src, ref dst } => digle.add_edge(src.clone(), dst.clone()),
        }
    }

    pub fn store_new_contents(&self, storage: &mut Storage) {
        if let Change::NewNode {
            ref id,
            ref contents,
        } = *self
        {
            storage.add_contents(id.clone(), contents.to_owned());
        }
    }

    pub fn set_patch_id(&mut self, new_id: &PatchId) {
        match *self {
            Change::NewNode { ref mut id, .. } => {
                id.set_patch_id(new_id);
            }
            Change::NewEdge {
                ref mut src,
                ref mut dst,
            } => {
                src.set_patch_id(new_id);
                dst.set_patch_id(new_id);
            }
            Change::DeleteNode { ref mut id } => {
                id.set_patch_id(new_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Change::*;
    use super::Changes;
    use crate::LineId;
    use crate::storage::File;
    use diff::LineDiff::*;

    #[test]
    fn from_diff_empty_first() {
        let file1 = File::from_bytes(b"");
        let file2 = File::from_bytes(b"something");
        let diff = vec![New(0)];

        let expected = vec![
            NewNode {
                id: LineId::cur(0),
                contents: b"something".to_vec(),
            },
        ];
        assert_eq!(Changes::from_diff(&file1, &file2, &diff).changes, expected);
    }
}

