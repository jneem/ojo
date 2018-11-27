use graph::Graph;
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

    fn make_back_edges(edges: &MMap<LineId, Edge>) -> MMap<LineId, Edge> {
        let mut ret = MMap::new();
        for (src, edge) in edges.iter() {
            ret.insert(edge.dest, Edge { dest: *src, pseudo: edge.pseudo });
        }
        ret
    }

    /// Brute-force conversion from a digle to its cached variant.
    ///
    /// In most cases, it should probably be faster to use the incremental updates, but this is
    /// useful for initial construction and also for testing.
    pub fn from_digle(digle: Digle) -> CachedDigleData {
        let deleted = digle.node_filtered(|u| !digle.is_live(u));
        let components = deleted.weak_components();
        let mut edges = MMap::new();

        // All edges between pairs of live nodes will just get copied into the cached digle.
        for u in digle.nodes().filter(|x| digle.is_live(x)) {
            for v in digle.out_neighbors(&u).filter(|x| digle.is_live(x)) {
                edges.insert(u, Edge { dest: v, pseudo: false });
            }
        }

        // Next we need to compute the pseudo-edges: which connectivity relations between live
        // nodes are implied by the deleted nodes?
        for part in components.parts() {
            let neighborhood = digle.neighbor_set(part.iter());
            let sub_digle = digle.node_filtered(|u| neighborhood.contains(u));

            // This is the collection of all live lines that are adjacent to a particular connected
            // component of deleted lines. We will compute the complete connectivity relation that
            // the deleted lines induce on these boundary lines, and then we will add a pseudo-edge
            // for each connected pair.
            let boundary = neighborhood.iter().filter(|u| digle.is_live(u));

            for u in boundary {
                for visit in sub_digle.dfs_from(u) {
                    match visit {
                        graph::dfs::Visit::Edge { dst, .. } => {
                            if digle.is_live(&dst) {
                                edges.insert(*u, Edge { dest: dst, pseudo: true });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // TODO: we could prune the pseudo-edges?

        let back_edges = Self::make_back_edges(&edges);
        CachedDigleData {
            lines: digle.lines().filter(|u| digle.is_live(u)).collect(),
            edges,
            back_edges,
            pending_changes: Vec::new(),
            pending_unchanges: Vec::new(),
        }
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
