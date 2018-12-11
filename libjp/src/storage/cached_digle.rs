use graph::Graph;
use multimap::MMap;
use partition::Partition;
use std::collections::BTreeSet as Set;
use std::collections::HashSet;

use crate::patch::Change;
use crate::storage::digle::Digle;
use crate::LineId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    pub dest: LineId,
    /// This is `true` whenever this is an edge that isn't present in the original digle.
    pub pseudo: bool,
}

// TODO: consider folding this into digle instead of having them separate.
// Part of the resolution algorithm can be done eagerly (e.g. deleting expired pseudo-edges,
// maintaining the partitions), and then just the bit about adding pseudo-edges can be done at the
// end.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "CachedDigle")]
pub(crate) struct CachedDigle {
    lines: Set<LineId>,
    edges: MMap<LineId, Edge>,
    back_edges: MMap<LineId, Edge>,
    pending_changes: Vec<Change>,
    pending_unchanges: Vec<Change>,

    // A partition of all the deleted nodes into weakly connected components.
    deleted_partition: Partition<LineId>,
    // A map from pseudo-edges (the forward-pointing ones only) to the set of parts (identified by
    // their representative) that are responsible for the pseudo-edge.
    pseudo_edge_reasons: MMap<(LineId, LineId), LineId>,
}

impl CachedDigle {
    pub fn new() -> CachedDigle {
        CachedDigle {
            lines: Set::new(),
            edges: MMap::new(),
            back_edges: MMap::new(),
            pending_changes: Vec::new(),
            pending_unchanges: Vec::new(),
            deleted_partition: Partition::new(),
            pseudo_edge_reasons: MMap::new(),
        }
    }

    // Goes through the pending changes and finds all of the lines that have been added to the
    // deleted set.
    fn pending_deleted_lines(&self, digle: Digle) -> HashSet<LineId> {
        self.pending_changes
            .iter()
            // First, find all the lines involved in a DeleteNode change...
            .filter_map(|ch| {
                match ch {
                    Change::DeleteNode { id } => Some(id),
                    _ => None,
                }
            })
            // ... but then actually check their status in the true digle, because they might have
            // been deleted and then undeleted again.
            .filter(|id| digle.has_line(id) && !digle.is_live(id))
            .cloned()
            .collect()
    }

    // Goes through the pending changes and finds all of the edges that have been added between
    // a pair of deleted lines.
    fn pending_dead_dead_edges(&self, digle: Digle) -> HashSet<(LineId, LineId)> {
        self.pending_changes
            .iter()
            .filter_map(|ch| {
                if let Change::NewEdge { src, dst } = ch {
                    Some((*src, *dst))
                } else {
                    None
                }
            })
            // Check to make sure the endpoints are still present but deleted.
            .filter(|(src, dst)|
                digle.has_line(src) && !digle.is_live(src)
                && digle.has_line(dst) && !digle.is_live(dst))
            .collect()
    }

    // `part` should be a non-empty connected component of the deleted nodes.
    fn add_all_pseudo_edges(&mut self, digle: Digle, part: &HashSet<LineId>) {
        let neighborhood = digle.neighbor_set(part.iter());

        // Find the representative of this connected component. The unwrap is ok because
        // `part` is non-empty.
        let part_rep = self.deleted_partition.representative(*part.iter().next().unwrap());
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
                            self.edges.insert(*u, Edge { dest: dst, pseudo: true });
                            self.back_edges.insert(dst, Edge { dest: *u, pseudo: true });
                            self.pseudo_edge_reasons.insert((*u, dst), part_rep);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Brute-force conversion from a digle to its cached variant.
    ///
    /// In most cases, it should probably be faster to use the incremental updates, but this is
    /// useful for initial construction and also for testing.
    pub fn from_digle(digle: Digle) -> CachedDigle {
        let deleted = digle.node_filtered(|u| !digle.is_live(u));
        let components = deleted.weak_components();
        let mut edges = MMap::new();
        let mut back_edges = MMap::new();
        let deleted_partition = components.parts()
            .map(|set| set.iter().cloned())
            .collect::<Partition<_>>();

        // All edges between pairs of live nodes will just get copied into the cached digle.
        for u in digle.nodes().filter(|x| digle.is_live(x)) {
            for v in digle.out_neighbors(&u).filter(|x| digle.is_live(x)) {
                edges.insert(u, Edge { dest: v, pseudo: false });
                back_edges.insert(v, Edge { dest: u, pseudo: false });
            }
        }

        let mut ret = CachedDigle {
            lines: digle.lines().filter(|u| digle.is_live(u)).collect(),
            edges,
            back_edges,
            pending_changes: Vec::new(),
            pending_unchanges: Vec::new(),
            deleted_partition,
            pseudo_edge_reasons: MMap::new(),
        };

        // Next we need to compute the pseudo-edges: which connectivity relations between live
        // nodes are implied by the deleted nodes?
        for part in components.parts() {
            ret.add_all_pseudo_edges(digle, part);
        }

        // TODO: we could prune the pseudo-edges?
        ret
    }

    // TODO: support iterating over only pseudo-edges
    pub fn out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.edges.get(line)
    }

    pub fn in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.back_edges.get(line)
    }

    pub fn apply_changes(&mut self, changes: &[Change]) {
        self.pending_changes.extend_from_slice(changes);
    }

    pub fn unapply_changes(&mut self, unchanges: &[Change]) {
        self.pending_unchanges.extend_from_slice(unchanges);
    }

    // The given reasons are out of date, so for every pseudoedge that touches the vertex u, remove
    // that reason from it (and delete the pseudoedge if there are no reasons remaining).
    fn remove_expired_pseudoedges(&mut self, u: &LineId, expired_reasons: &HashSet<LineId>) {
        let mut to_delete = Vec::new();

        // Find all expired reasons for pseudo-edges pointing out of u.
        for e in self.out_edges(u) {
            if e.pseudo {
                for reason in self.pseudo_edge_reasons.get(&(*u, e.dest)) {
                    if expired_reasons.contains(reason) {
                        to_delete.push((*u, e.dest, *reason));
                    }
                }
            }
        }
        // Find all expired reasons for pseudo-edges pointing into u.
        for e in self.in_edges(u) {
            if e.pseudo {
                // Note that reasons are always stored as forward edges, not back edges, so we need
                // to reverse the order of the edge here.
                for reason in self.pseudo_edge_reasons.get(&(e.dest, *u)) {
                    if expired_reasons.contains(reason) {
                        to_delete.push((e.dest, *u, *reason));
                    }
                }
            }
        }

        for (u, v, reason) in to_delete {
            self.pseudo_edge_reasons.remove(&(u, v), &reason);
            // If that was the last reason, delete the edge.
            if self.pseudo_edge_reasons.get(&(u, v)).next().is_none() {
                self.edges.remove(&u, &Edge { dest: v, pseudo: true });
                self.back_edges.remove(&v, &Edge { dest: u, pseudo: true });
            }
        }
    }

    /// Goes through all the pending changes (and unchanges), and actually does the work.
    pub fn resolve(&mut self, digle: Digle) {
        let new_deleted_lines = self.pending_deleted_lines(digle);
        let new_deleted_edges = self.pending_dead_dead_edges(digle);

        // TODO: need to explain that not everything in here is necessarily a representative in the
        // old partition, but that for every part that was changed, its old representative will be
        // in here.
        let mut expired_reasons = HashSet::new();

        // This will contain at least one element from every part in the partition that was
        // modified. These are the parts where we will need to re-check the connectivity relation.
        let mut touched_parts = HashSet::new();
        for line in &new_deleted_lines {
            self.deleted_partition.insert(*line);
        }
        for &(src, dst) in &new_deleted_edges {
            let src_rep = self.deleted_partition.representative_mut(src);
            let dst_rep = self.deleted_partition.representative_mut(dst);
            if self.deleted_partition.merge(src, dst) {
                expired_reasons.insert(src_rep);
                expired_reasons.insert(dst_rep);
                touched_parts.insert(src);
            }
        }

        // Finds the (new) representatives of all the touched parts.
        let touched_reps = touched_parts.iter()
            .map(|elt| self.deleted_partition.representative(*elt))
            .collect::<HashSet<_>>();

        // Finds all the extant but deleted lines that are in any touched part.
        let touched_elts = touched_reps.iter()
            .flat_map(|tp| self.deleted_partition.iter_part(*tp))
            .filter(|elt| digle.has_line(elt) && !digle.is_live(elt))
            .collect::<HashSet<_>>();

        let touched_nbhd = digle.neighbor_set(touched_elts.iter());
        let touched_bdy = touched_nbhd.iter().filter(|u| digle.is_live(u));
        for u in touched_bdy {
            self.remove_expired_pseudoedges(u, &expired_reasons);
        }

        // Now we should recompute all the connectivity relations for everything on the boundary of
        // a touched part. This will tell us which pesudo-edges to add.
        let touched_subgraph = digle.node_filtered(|u| touched_elts.contains(u));
        let components = touched_subgraph.weak_components();
        for part in components.parts() {
            self.add_all_pseudo_edges(digle, part);
        }
    }
}

// TODO: figure out how to test all this.
// - generate changes from a digle
// - figure out the invariants
// - add a method to validate (i.e. check that it agrees with the original digle)
