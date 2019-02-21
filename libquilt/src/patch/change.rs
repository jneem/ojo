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

use quilt_diff::LineDiff;

use crate::storage::File;
use crate::{NodeId, PatchId};

/// A set of [`Change`]s.
///
/// This is basically the ``meat'' of a [`Patch`](crate::Patch); everthing else is metadata.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Changes {
    /// The list of [`Change`]s.
    ///
    /// NOTE: this may become private in the future.
    pub changes: Vec<Change>,
}

// This is for creating `Changes` from diffs.  While iterating through the diff, we need to
// remember what the previous line was and where it came from: either there wasn't one, or it came
// from one of the two files.
enum LastLine<'a> {
    Start,
    File1(&'a NodeId),
    File2(&'a NodeId),
}

impl<'a> LastLine<'a> {
    fn either(&self) -> Option<&NodeId> {
        match *self {
            LastLine::File1(i) => Some(i),
            LastLine::File2(i) => Some(i),
            LastLine::Start => None,
        }
    }
}

impl Changes {
    /// Converts a [`diff::LineDiff`] into a set of changes.
    ///
    /// The two `File` arguments should be the same ones (in the same order) as those that were
    /// used to create the diff.
    pub fn from_diff(file1: &File, file2: &File, diff: &[LineDiff]) -> Changes {
        let mut changes = Vec::new();
        let mut last = LastLine::Start;
        for d in diff {
            match *d {
                LineDiff::New(i) => {
                    let id = file2.node_id(i);
                    changes.push(Change::NewNode {
                        id: *id,
                        contents: file2.node(i).to_owned(),
                    });

                    // We are adding a new line, so we need to connect it to whatever line came
                    // before it, no matter where it came from.
                    if let Some(last_id) = last.either() {
                        changes.push(Change::NewEdge {
                            src: *last_id,
                            dest: *id,
                        });
                    }
                    last = LastLine::File2(id);
                }
                LineDiff::Keep(i, _) => {
                    let id = file1.node_id(i);

                    // If the last line came from the new file, we need to hook it up to this line.
                    if let LastLine::File2(last_id) = last {
                        changes.push(Change::NewEdge {
                            src: *last_id,
                            dest: *id,
                        });
                    }
                    last = LastLine::File1(id);
                }
                LineDiff::Delete(i) => {
                    let id = file1.node_id(i);
                    changes.push(Change::DeleteNode { id: *id });
                }
            }
        }
        Changes { changes }
    }

    /// Modifies all of the changes in this changeset to have the given [`PatchId`].
    pub fn set_patch_id(&mut self, new_id: &PatchId) {
        for ch in &mut self.changes {
            ch.set_patch_id(new_id);
        }
    }
}

/// A single change.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Change {
    /// A change which adds a new node to the graggle, with an ID that must be unique, and with the
    /// given contents.
    NewNode {
        /// The ID of the new node.
        id: NodeId,
        /// The contents of the new node.
        contents: Vec<u8>,
    },
    /// Marks a node as deleted. Note that deleted nodes are never actually removed; they remain
    /// but they are simply marked as deleted.
    DeleteNode {
        /// The ID of the node to delete.
        id: NodeId,
    },
    /// Adds a new edge (i.e. a new ordering relation) between two nodes. Those nodes must either
    /// already exist in the graggle at the time this change is applied. (If this `Change` is part of
    /// a `Changes` that adds some nodes and also an edge between them, then that's ok too.)
    NewEdge {
        /// The source of the new edge (i.e. the one that comes first in the ordering).
        src: NodeId,
        /// The destination of the new edge.
        dest: NodeId,
    },
}

impl Change {
    // Modifies the PatchId of this Change.
    fn set_patch_id(&mut self, new_id: &PatchId) {
        match *self {
            Change::NewNode { ref mut id, .. } => {
                id.set_patch_id(new_id);
            }
            Change::NewEdge {
                ref mut src,
                ref mut dest,
            } => {
                src.set_patch_id(new_id);
                dest.set_patch_id(new_id);
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
    use crate::storage::File;
    use crate::NodeId;
    use quilt_diff::LineDiff::*;

    #[test]
    fn from_diff_empty_first() {
        let file1 = File::from_bytes(b"");
        let file2 = File::from_bytes(b"something");
        let diff = vec![New(0)];

        let expected = vec![NewNode {
            id: NodeId::cur(0),
            contents: b"something".to_vec(),
        }];
        assert_eq!(Changes::from_diff(&file1, &file2, &diff).changes, expected);
    }
}
