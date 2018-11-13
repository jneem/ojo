use multimap::MMap;
use std::collections::BTreeSet as Set;

use crate::patch::Change;
use crate::storage::digle::Digle;
use crate::LineId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    pub dest: LineId,
    /// This is `true` whenever this is an edge that isn't present in the original digle.
    pub pseudo: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "CachedDigle")]
pub(crate) struct CachedDigleData {
    lines: Set<LineId>,
    edges: MMap<LineId, Edge>,
    back_edges: MMap<LineId, Edge>,
    pending_changes: Vec<Change>,
    pending_unchanges: Vec<Change>,
}

impl CachedDigleData {
    pub fn new() -> CachedDigleData {
        CachedDigleData {
            lines: Set::new(),
            edges: MMap::new(),
            back_edges: MMap::new(),
            pending_changes: Vec::new(),
            pending_unchanges: Vec::new(),
        }
    }

    /// Brute-force conversion from a digle to its cached variant.
    ///
    /// In most cases, it should probably be faster to use the incremental updates, but this is
    /// useful for initial construction and also for testing.
    pub fn from_digle(digle: Digle) -> CachedDigleData {
        unimplemented!()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CachedDigle<'a> {
    data: &'a CachedDigleData,
}

impl<'a> CachedDigle<'a> {
    pub fn out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data.edges.get(line)
    }

    pub fn in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data.back_edges.get(line)
    }
}

#[derive(Debug)]
pub struct CachedDigleMut<'a> {
    data: &'a mut CachedDigleData,
}

impl<'a> CachedDigleMut<'a> {
    pub fn apply_changes(&mut self, changes: &[Change]) {
        self.data.pending_changes.extend_from_slice(changes);
    }

    pub fn apply_unchanges(&mut self, unchanges: &[Change]) {
        self.data.pending_unchanges.extend_from_slice(unchanges);
    }

    /// Goes through all the pending changes (and unchanges), and actually does the work.
    pub fn resolve(&mut self) {
        unimplemented!();
    }
}
