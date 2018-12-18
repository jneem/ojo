use graph::Graph;
use multimap::MMap;
use partition::Partition;
use std::collections::BTreeSet as Set;
use std::collections::HashSet;

use crate::LineId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EdgeKind {
    Live,
    Pseudo,
    // The order here is important: by putting deleted edges last, we can efficiently ignore them:
    // if we iterate through the neighbors of node but stop at the first deleted one, then we've
    // ignored all of the deleted neighbors.
    Deleted,
}

impl EdgeKind {
    fn from_deleted(deleted: bool) -> EdgeKind {
        if deleted {
            EdgeKind::Deleted
        } else {
            EdgeKind::Live
        }
    }
}

/// This struct represents a directed edge in a digle graph.
///
/// Note that we don't actually store the source line, only the destination. However, the main way
/// of getting access to an `Edge` is via the `Digle::out_edges` or `Digle::in_edges` functions, so
/// usually you will only encounter an `Edge` if you already know what the source line is.
///
/// Note that edges are ordered, and that live edges will always come before deleted edges. This
/// helps ensure quick access to live edges.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    pub kind: EdgeKind,
    /// The destination of this (directed) edge.
    pub dest: LineId,
}

impl Edge {
    fn not_deleted(&self) -> bool {
        self.kind != EdgeKind::Deleted
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "Digle")]
pub(crate) struct DigleData {
    lines: Set<LineId>,
    deleted_lines: Set<LineId>,
    edges: MMap<LineId, Edge>,
    back_edges: MMap<LineId, Edge>,

    // A partition of all the deleted nodes into weakly connected components.
    deleted_partition: Partition<LineId>,
    // A map from pseudo-edges (the forward-pointing ones only) to the set of parts (identified by
    // their representative) that are responsible for the pseudo-edge.
    pseudo_edge_reasons: MMap<(LineId, LineId), LineId>,
    // A map from "reasons" (i.e. representatives of a partition) to edges that are there because
    // of that reason.
    reason_pseudo_edges: MMap<LineId, (LineId, LineId)>,
    // These are the component representatives whose components are dirty (i.e. we need to
    // recalculate the connectedness relation that they induce).
    dirty_reps: Set<LineId>,
}

impl DigleData {
    pub fn new() -> DigleData {
        DigleData {
            lines: Set::new(),
            deleted_lines: Set::new(),
            edges: MMap::new(),
            back_edges: MMap::new(),
            deleted_partition: Partition::new(),
            pseudo_edge_reasons: MMap::new(),
            reason_pseudo_edges: MMap::new(),
            dirty_reps: Set::new(),
        }
    }

    pub fn as_digle<'a>(&'a self) -> Digle<'a> {
        Digle { data: self }
    }

    pub fn all_out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.edges.get(line)
    }

    pub fn all_in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.back_edges.get(line)
    }

    pub fn add_node(&mut self, id: LineId) {
        self.lines.insert(id);
    }

    // Deletes an edge (both forward and back), but does nothing else to ensure consistency and
    // maintain invariants.
    fn internal_delete_edge(&mut self, src: &LineId, edge: &Edge) {
        self.edges.remove(src, edge);
        let back_edge = Edge {
            dest: *src,
            kind: edge.kind,
        };
        self.back_edges.remove(&edge.dest, &back_edge);
    }

    fn internal_delete_back_edge(&mut self, dest: &LineId, back_edge: &Edge) {
        self.back_edges.remove(dest, back_edge);
        let edge = Edge {
            dest: *dest,
            kind: back_edge.kind,
        };
        self.edges.remove(&edge.dest, &edge);
    }

    pub fn unadd_node(&mut self, id: &LineId) {
        // If we are unadding a line, it means we are unapplying the patch in which the line was
        // introduced. Since we must have already unapplied any reverse-dependencies of the patch,
        // the line must be live (it can't have been marked as deleted).
        assert!(self.lines.contains(id));
        self.lines.remove(id);

        // Remove all the edges that had anything to do with this node. (When unapplying a patch,
        // most of the edges would probably have already been deleted, but there might be lingering
        // pseudo-edges.)
        let out_edges = self.all_out_edges(id).cloned().collect::<Vec<_>>();
        let in_edges = self.all_in_edges(id).cloned().collect::<Vec<_>>();
        for e in out_edges {
            self.internal_delete_edge(id, &e);
        }
        for e in in_edges {
            self.internal_delete_back_edge(id, &e);
        }

        // Because we just unadded a line that was live, it can't have any effect on pseudo-edges,
        // so no need to update them.
    }

    /// Given a live node, marks it as deleted. That is, the node doesn't vanish; it turns into a
    /// tombstone.
    ///
    /// # Panics
    /// Panics if the node doesn't exist, or if exists but is not live.
    pub fn delete_node(&mut self, id: &LineId) {
        assert!(self.lines.contains(id));
        self.lines.remove(id);
        self.deleted_lines.insert(id.clone());
        self.deleted_partition.insert(id.clone());

        // All the edges (both forward and backwards) pointing towards the newly deleted node need
        // to be marked as deleted.
        let out_neighbors = self.all_out_edges(id).cloned().collect::<Vec<_>>();
        let in_neighbors = self.all_in_edges(id).cloned().collect::<Vec<_>>();
        for e in out_neighbors {
            self.delete_opposite_edge(id, &e, true);
        }
        for e in in_neighbors {
            self.delete_opposite_edge(id, &e, false);
        }
    }

    pub fn undelete_node(&mut self, id: &LineId) {
        assert!(self.deleted_lines.contains(id));
        self.deleted_lines.remove(id);
        self.lines.insert(id.clone());

        // All the edges (both forward and backwards) pointing towards the newly deleted node need
        // to be marked as live.
        let out_neighbors = self.all_out_edges(id).cloned().collect::<Vec<_>>();
        let in_neighbors = self.all_in_edges(id).cloned().collect::<Vec<_>>();
        for e in out_neighbors {
            self.undelete_opposite_edge(id, &e, true);
        }
        for e in in_neighbors {
            self.undelete_opposite_edge(id, &e, false);
        }

        // Mark the entire connected component containing `id` as dirty. Note that we don't
        // actually remove `id` from the component, because it might take too long to compute how
        // the component splits up. When it comes time to compute the new connectivity relation, we
        // will figure out how the component splits.
        self.mark_dirty(id);
    }

    // The node `src` has just been deleted, and `edge` is an edge pointing out from it (either
    // forwards or backwards). We want to delete the edge pointing from edge.dest to src.
    fn delete_opposite_edge(&mut self, src: &LineId, edge: &Edge, edge_points_forwards: bool) {
        // This is the edge_map that points in the opposite direction as `edge`.
        let opposite_edges = if edge_points_forwards {
            &mut self.back_edges
        } else {
            &mut self.edges
        };

        if edge.kind == EdgeKind::Pseudo {
            // Pseudo-edges don't get marked as deleted, they just get removed.
            let opposite_edge = Edge {
                dest: *src,
                kind: EdgeKind::Pseudo,
            };
            opposite_edges.remove(&edge.dest, &opposite_edge);
        } else {
            // To mark the edge as deleted, we actually remove it and then add it back in again
            // (because deleted edges appear in a different position in the map).
            let mut opposite_edge = Edge {
                dest: *src,
                kind: EdgeKind::Live,
            };
            opposite_edges.remove(&edge.dest, &opposite_edge);
            opposite_edge.kind = EdgeKind::Deleted;
            opposite_edges.insert(edge.dest, opposite_edge);
        }

        // The node `src` was just deleted. If `edge.dest` is also deleted, it means that they now
        // belong to the same connected component of deleted edges.
        if edge.kind == EdgeKind::Deleted {
            self.merge_components(src, &edge.dest);
        }
    }

    // The node `src` was just undeleted, and `edge` points out from `src`.
    fn undelete_opposite_edge(&mut self, src: &LineId, edge: &Edge, edge_points_forwards: bool) {
        // This is the edge_map that points in the opposite direction as `edge`.
        let opposite_edges = if edge_points_forwards {
            &mut self.back_edges
        } else {
            &mut self.edges
        };

        // Unlike `delete_opposite_edge`, there's no change of encountering a pseudo-edge pointing
        // from `edge.dest` to `src` (because `src` was just undeleted, and while it was deleted no
        // pseudo-edges pointed at it).
        let mut opposite_edge = Edge {
            dest: *src,
            kind: EdgeKind::Deleted,
        };
        opposite_edges.remove(&edge.dest, &opposite_edge);
        opposite_edge.kind = EdgeKind::Live;
        opposite_edges.insert(edge.dest, opposite_edge);

        // Unlike in `delete_opposite_edge`, there's no need here to do anything about pseudo-edges
        // and partition-merging. That's because the entire partition that `src` used to belong to
        // has already been marked as dirty.
    }

    // `id` and `other` are two deleted nodes that have just been connected by an edge. We need to
    // mark them as being in the same connected component of deleted nodes. This also entails
    // marking the merged component as dirty, and removing any obsolete pseudo-edges.
    fn merge_components(&mut self, id1: &LineId, id2: &LineId) {
        let rep1 = self.deleted_partition.representative(*id1);
        let rep2 = self.deleted_partition.representative(*id2);
        self.deleted_partition.merge(rep1, rep2);
        let new_rep = self.deleted_partition.representative(rep1);

        self.delete_obsolete_reason(&rep1);
        self.delete_obsolete_reason(&rep2);

        self.dirty_reps.remove(&rep1);
        self.dirty_reps.remove(&rep2);
        self.dirty_reps.insert(new_rep);
    }

    // `reason` was (and possibly still is) the representative of a component that got modified. We
    // can't trust any pseudo-edges coming from that component, so delete them all.
    fn delete_obsolete_reason(&mut self, reason: &LineId) {
        let obsolete_pairs = self
            .reason_pseudo_edges
            .get(reason)
            .cloned()
            .collect::<Vec<_>>();

        for (src, dest) in obsolete_pairs {
            let e = Edge {
                dest: dest,
                kind: EdgeKind::Pseudo,
            };
            self.internal_delete_edge(&src, &e);
            self.pseudo_edge_reasons.remove(&(src, dest), reason);
        }
        self.reason_pseudo_edges.remove_all(reason);
    }

    // Marks the component containing `id` as dirty.
    fn mark_dirty(&mut self, id: &LineId) {
        self.dirty_reps
            .insert(self.deleted_partition.representative(*id));
    }

    pub fn add_edge(&mut self, from: LineId, to: LineId) {
        let from_deleted = !self.lines.contains(&from);
        let to_deleted = !self.lines.contains(&to);
        assert!(!from_deleted || self.deleted_lines.contains(&from));
        assert!(!to_deleted || self.deleted_lines.contains(&to));

        self.edges.insert(
            from.clone(),
            Edge {
                kind: EdgeKind::from_deleted(to_deleted),
                dest: to.clone(),
            },
        );
        self.back_edges.insert(
            to,
            Edge {
                kind: EdgeKind::from_deleted(from_deleted),
                dest: from,
            },
        );

        if from_deleted && to_deleted {
            self.merge_components(&from, &to);
        } else if from_deleted {
            self.mark_dirty(&from);
        } else if to_deleted {
            self.mark_dirty(&to);
        }
    }

    pub fn resolve_pseudo_edges(&mut self) {
        let mut dirty_reps = Set::new();
        std::mem::swap(&mut dirty_reps, &mut self.dirty_reps);

        // Each partition represented by a dirty rep needs to be rechecked, because it's possible
        // that it actually encompasses multiple connected components in the new digle.
        let digle = self.as_digle();
        let sub_graph = digle.node_filtered(|u| {
            !digle.is_live(u) && dirty_reps.contains(&self.deleted_partition.representative(*u))
        });
        let components = sub_graph.weak_components().into_parts();

        // Remove all the messed up parts from the partition; later we'll re-add them from the
        // connected components we just computed.
        for rep in dirty_reps {
            self.deleted_partition.remove_part(rep);
        }

        // Add in the required pseudo-edges and fix up the partition.
        for component in components {
            self.add_component_pseudo_edges(&component);

            // Add everything in the current component as a new component in deleted_partition.
            let mut iter = component.into_iter();
            // Unwrap is ok because the components are guaranteed to be non-empty.
            let rep = iter.next().unwrap();
            self.deleted_partition.insert(rep);
            for u in iter {
                self.deleted_partition.insert(u);
                self.deleted_partition.merge(rep, u);
            }
        }
    }

    /// # Panics
    ///
    /// Panics unless `from` and `to` are lines in this digle. In particular, if you're planning to
    /// remove some lines and the edge between them, you need to remove the edge first.
    pub fn unadd_edge(&mut self, from: &LineId, to: &LineId) {
        let from_deleted = !self.lines.contains(&from);
        let to_deleted = !self.lines.contains(&to);
        assert!(!from_deleted || self.deleted_lines.contains(&from));
        assert!(!to_deleted || self.deleted_lines.contains(&to));

        let forward_edge = Edge {
            kind: EdgeKind::from_deleted(to_deleted),
            dest: to.clone(),
        };
        let back_edge = Edge {
            kind: EdgeKind::from_deleted(from_deleted),
            dest: from.clone(),
        };
        self.edges.remove(&from, &forward_edge);
        self.back_edges.remove(&to, &back_edge);
    }

    // Adds all the pseudo-edges that are induced by a single connected component of deleted nodes.
    //
    // `component` must be a non-empty connected component of the deleted nodes.
    fn add_component_pseudo_edges(&mut self, component: &HashSet<LineId>) {
        let digle = self.as_digle();
        let neighborhood = digle.neighbor_set(component.iter());

        // Find the representative of this connected component. The unwrap is ok because
        // `component` is non-empty.
        let rep = self
            .deleted_partition
            .representative(*component.iter().next().unwrap());
        let sub_digle = digle.node_filtered(|u| neighborhood.contains(u));

        // This is the collection of all live lines that are adjacent to a particular connected
        // component of deleted lines. We will compute the complete connectivity relation that
        // the deleted lines induce on these boundary lines, and then we will add a pseudo-edge
        // for each connected pair.
        let boundary = neighborhood.iter().filter(|u| digle.is_live(u));

        let mut pairs = Vec::new();
        for u in boundary {
            for visit in sub_digle.dfs_from(u) {
                match visit {
                    graph::dfs::Visit::Edge { dst, .. } => {
                        if digle.is_live(&dst) {
                            pairs.push((*u, dst));
                        }
                    }
                    _ => {}
                }
            }
        }
        for (src, dest) in pairs {
            self.edges.insert(
                src,
                Edge {
                    dest,
                    kind: EdgeKind::Pseudo,
                },
            );
            self.back_edges.insert(
                dest,
                Edge {
                    dest: src,
                    kind: EdgeKind::Pseudo,
                },
            );
            self.pseudo_edge_reasons.insert((src, dest), rep);
            self.reason_pseudo_edges.insert(rep, (src, dest));
        }
    }
}

// This wrapping is a bit annoying. It would be simpler just to rename `DigleData` to `Digle` and
// then pass around `&Digle`s. The thing is that we want to implement `Graph` for `&Digle`, and I
// had some problems with that for some reason (can no longer remember why...). Certainly, the lack
// of ATCs means we can't implement `Graph` for `Digle`.
#[derive(Clone, Copy, Debug)]
pub struct Digle<'a> {
    data: &'a DigleData,
}

impl<'a> Digle<'a> {
    pub fn lines<'b>(&'b self) -> impl Iterator<Item = LineId> + 'b {
        self.data.lines.iter().cloned()
    }

    pub fn out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data.edges.get(line).take_while(|e| e.not_deleted())
    }

    pub fn all_out_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data.edges.get(line)
    }

    pub fn in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data
            .back_edges
            .get(line)
            .take_while(|e| e.not_deleted())
    }

    pub fn all_in_edges<'b>(&'b self, line: &LineId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.data.back_edges.get(line)
    }

    pub fn has_line(&self, line: &LineId) -> bool {
        self.data.lines.contains(line) || self.data.deleted_lines.contains(line)
    }

    pub fn is_live(&self, line: &LineId) -> bool {
        assert!(self.has_line(line));
        self.data.lines.contains(line)
    }

    pub fn assert_consistent(&self) {
        // The live and deleted lines should be disjoint.
        assert!(self.data.lines.is_disjoint(&self.data.deleted_lines));

        let node_exists = |id| self.data.lines.contains(id) || self.data.deleted_lines.contains(id);
        // The source and destination of every edge should exist somewhere.
        // The destination should be deleted if and only if the edge kind is `Deleted`.
        // There should be a one-to-one correspondence between edges and back_edges.
        let mut seen_back_edges = HashSet::new();
        for (src, edge) in self.data.edges.iter() {
            assert!(node_exists(src));
            assert!(node_exists(&edge.dest));
            assert_eq!(
                self.data.deleted_lines.contains(&edge.dest),
                edge.kind == EdgeKind::Deleted
            );

            let back_edge = Edge {
                dest: *src,
                kind: if edge.kind == EdgeKind::Pseudo {
                    EdgeKind::Pseudo
                } else {
                    EdgeKind::from_deleted(self.data.deleted_lines.contains(src))
                },
            };
            assert!(self.data.back_edges.contains(&edge.dest, &back_edge));
            seen_back_edges.insert((edge.dest, back_edge));
        }
        // We've checked that every forward edge corresponds to a backward edge; now check that
        // every backward edge was encountered in this way.
        for (src, back_edge) in self.data.back_edges.iter() {
            assert!(seen_back_edges.contains(&(*src, *back_edge)));
        }

        // TODO:
        // - check that deleted_partition is indeed a partition of the deleted nodes into
        //   connected components.
        // - check that the ordering induced by the pseudo-edges is the right one
    }
}

impl<'a> From<&'a DigleData> for Digle<'a> {
    fn from(d: &'a DigleData) -> Digle<'a> {
        Digle { data: d }
    }
}

#[derive(Debug)]
pub struct DigleMut<'a> {
    data: &'a mut DigleData,
}

impl<'a> DigleMut<'a> {
    pub fn as_digle<'b>(&'b self) -> Digle<'b> {
        Digle { data: self.data }
    }

    pub fn add_node(&mut self, id: LineId) {
        self.data.add_node(id);
    }

    pub fn unadd_node(&mut self, id: &LineId) {
        self.data.unadd_node(id);
    }

    pub fn delete_node(&mut self, id: &LineId) {
        self.data.delete_node(id);
    }

    pub fn undelete_node(&mut self, id: &LineId) {
        self.data.undelete_node(id);
    }

    pub fn add_edge(&mut self, from: LineId, to: LineId) {
        self.data.add_edge(from, to);
    }

    /// # Panics
    ///
    /// Panics unless `from` and `to` are lines in this digle. In particular, if you're planning to
    /// remove some lines and the edge between them, you need to remove the lines first.
    pub fn unadd_edge(&mut self, from: &LineId, to: &LineId) {
        self.data.unadd_edge(from, to);
    }
}

impl<'a> From<&'a mut DigleData> for DigleMut<'a> {
    fn from(d: &'a mut DigleData) -> DigleMut<'a> {
        DigleMut { data: d }
    }
}

impl<'a> graph::Graph for Digle<'a> {
    type Node = LineId;
    type Edge = LineId;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = Self::Node> + 'b> {
        Box::new(
            self.data
                .lines
                .iter()
                .chain(self.data.deleted_lines.iter())
                .cloned(),
        )
    }

    fn out_edges<'b>(&'b self, u: &LineId) -> Box<dyn Iterator<Item = Self::Node> + 'b> {
        Box::new(self.all_out_edges(u).map(|e| &e.dest).cloned())
    }

    fn in_edges<'b>(&'b self, u: &LineId) -> Box<dyn Iterator<Item = Self::Node> + 'b> {
        Box::new(self.all_in_edges(u).map(|e| &e.dest).cloned())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{LineId, PatchId};
    use crate::patch::{Change, Changes};
    use super::*;

    use byteorder::{LittleEndian, WriteBytesExt};
    use proptest::prelude::*;
    use proptest::collection::hash_set;
    use proptest::sample::subsequence;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // When generating digles, we could in principle put in as many as n^2 edges, but that's way
    // too many to be realistic (a realistic value would be around 2). So we allow only up to
    // n*MAX_AVG_DEGREE.
    const MAX_AVG_DEGREE: usize = 5;

    prop_compose! {
        // Creates an arbitrary digle with no deleted nodes.
        fn arb_live_digle(max_nodes: usize)
                         (num_nodes in 1..max_nodes)
                         (edges in hash_set((0..num_nodes, 0..num_nodes), 0..(num_nodes * MAX_AVG_DEGREE)),
                          num_nodes in Just(num_nodes))
                         -> DigleData
        {
            let mut ret = DigleData::new();
            for i in 0..num_nodes {
                ret.lines.insert(LineId::cur(i as u64));
            }
            for (u, v) in edges {
                if u != v {
                    let u = LineId::cur(u as u64);
                    let v = LineId::cur(v as u64);
                    ret.edges.insert(u, Edge { dest: v, kind: EdgeKind::Live });
                    ret.back_edges.insert(v, Edge { dest: u, kind: EdgeKind::Live });
                }
            }
            ret
        }
    }

    // When we create different `Changes`, we need to give each one a unique PatchId. We achieve
    // this by simply incrementing a counter. We start from 1, because by default the digles that
    // we create use the id 0.
    static CUR_ID: AtomicUsize = AtomicUsize::new(1);

    // Create arbitrary patches on top of digles. Basically, an arbitrary patch consists of an
    // arbitrary subset of nodes to delete, and an arbitrary set of nodes to add, with arbitrary
    // edges between the new nodes, and also between the new nodes and the old ones.
    fn arb_changes<'a>(digle: &'a DigleData, size: usize) -> BoxedStrategy<Changes> {
        fn make_changes(
            old_ids: Vec<LineId>,
            nodes_to_delete: Vec<LineId>,
            num_to_add: usize,
            new_new_edges: HashSet<(usize, usize)>,
            new_old_edges: HashSet<(usize, usize)>,
            old_new_edges: HashSet<(usize, usize)>,
        ) -> Changes
        {
            let patch_id_int = CUR_ID.fetch_add(1, Ordering::SeqCst);
            let mut patch_id = PatchId::cur();
            (&mut patch_id.data[..]).write_u64::<LittleEndian>(patch_id_int as u64).unwrap();

            let new_ids = (0..num_to_add)
                .map(|i| LineId { patch: patch_id, line: i as u64 })
                .collect::<Vec<_>>();

            let deletions = nodes_to_delete.iter()
                .map(|u| Change::DeleteNode { id: *u });

            let insertions = new_ids.iter()
                .map(|u| Change::NewNode { id: *u, contents: vec![] });

            let edges =
                new_new_edges.into_iter().map(|(i, j)| (new_ids[i], new_ids[j]))
                .chain(new_old_edges.into_iter().map(|(i, j)| (new_ids[i], old_ids[j])))
                .chain(old_new_edges.into_iter().map(|(i, j)| (old_ids[i], new_ids[j])))
                .filter(|(u, v)| u != v);
            let edges = edges
                .map(|(u, v)| Change::NewEdge { src: u, dst: v });

            let changes = deletions.chain(insertions).chain(edges).collect::<Vec<_>>();
            Changes { changes }
        }

        let old_ids = digle.lines.iter().cloned().collect::<Vec<_>>();
        let num_to_add = 1..size;

        // Strategy returning a tuple
        // (nodes_to_delete, num_to_add, new_new_edges, new_old_edges, old_new_edges)
        let old = old_ids.clone();
        let changes = num_to_add.prop_flat_map(move |n|
            (subsequence(old.clone(), 0..old.len()),
             Just(n),
             hash_set((0..n, 0..n), 0..(MAX_AVG_DEGREE * n)),
             hash_set((0..n, 0..old.len()), 0..(MAX_AVG_DEGREE * n.min(old.len()))),
             hash_set((0..old.len(), 0..n), 0..(MAX_AVG_DEGREE * n.min(old.len()))),
             )
            );
        changes.prop_map(move |(del, n, nn, no, on)| make_changes(old_ids.clone(), del, n, nn, no, on)).boxed()
    }

    // Creates an arbitrary digle and a change that can be applied to it.
    fn arb_digle_and_change(initial_size: usize, change_size: usize) -> BoxedStrategy<(DigleData, Changes)> {
        let digle = arb_live_digle(initial_size);
        digle.prop_flat_map(move |d| {
            let ch = arb_changes(&d, change_size);
            (Just(d), ch)
        }).boxed()
    }

    proptest! {
        #[test]
        fn live_digles_consistent(ref d in arb_live_digle(20)) {
            d.as_digle().assert_consistent();
        }
    }

    fn apply_changes(digle: &mut DigleData, changes: &Changes) {
        for ch in &changes.changes {
            match *ch {
                Change::NewNode { ref id, .. } => digle.add_node(id.clone()),
                Change::DeleteNode { ref id } => digle.delete_node(&id),
                Change::NewEdge { ref src, ref dst } => digle.add_edge(src.clone(), dst.clone()),
            }
        }
    }

    proptest! {
        #[test]
        fn digle_then_change((ref d, ref ch) in arb_digle_and_change(20, 10)) {
            let mut d = d.clone();
            d.as_digle().assert_consistent();
            apply_changes(&mut d, ch);
            d.as_digle().assert_consistent();
        }
    }
}
