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

use graph::Graph;
use multimap::MMap;
use partition::Partition;
use std::collections::BTreeSet as Set;
use std::collections::HashSet;

use crate::{NodeId, PatchId};

/// The different kinds of edges.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EdgeKind {
    /// This edge points to a live node.
    Live,
    /// This edge was not present in the original graggle. It was added as an optimization, to skip
    /// over deleted nodes. (TODO: reference some more detailed docs on pseudo-edges)
    Pseudo,
    // The order here is important: by putting deleted edges last, we can efficiently ignore them:
    // if we iterate through the neighbors of node but stop at the first deleted one, then we've
    // ignored all of the deleted neighbors.
    /// This edges points to a deleted node.
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

/// This struct represents a directed edge in a graggle graph.
///
/// Note that we don't actually store the source node, only the destination. However, the main way
/// of getting access to an `Edge` is via the `Graggle::out_edges` or `Graggle::in_edges` functions, so
/// usually you will only encounter an `Edge` if you already know what the source node is.
///
/// Note that edges are ordered, and that live edges will always come before deleted edges. This
/// helps ensure quick access to live edges.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Edge {
    /// What kind of edge is it?
    pub kind: EdgeKind,
    /// The destination of this (directed) edge.
    pub dest: NodeId,
    /// Which patch introduced this edge?
    ///
    /// If this is a pseudo-edge, then this field will be "blank", meaning that it will be set to
    /// `PatchId::cur`.
    ///
    /// This field is necessary because of the possiblity that two different patches will add the
    /// same edge. If this happens and then one of the patches is unapplied, we'd better make sure
    /// to still have an edge present afterwards.
    pub patch: PatchId,
}

impl Edge {
    fn not_deleted(&self) -> bool {
        self.kind != EdgeKind::Deleted
    }

    fn new_pseudo(dest: NodeId) -> Edge {
        Edge {
            dest: dest,
            kind: EdgeKind::Pseudo,
            patch: PatchId::cur(),
        }
    }

    fn new_live(dest: NodeId, patch: PatchId) -> Edge {
        Edge {
            dest,
            kind: EdgeKind::Live,
            patch,
        }
    }

    fn new_deleted(dest: NodeId, patch: PatchId) -> Edge {
        Edge {
            dest,
            kind: EdgeKind::Deleted,
            patch,
        }
    }

    // "Real" means either live or deleted, but not pseudo
    fn new_real(dest: NodeId, deleted: bool, patch: PatchId) -> Edge {
        Edge {
            dest,
            kind: EdgeKind::from_deleted(deleted),
            patch,
        }
    }
}

impl graph::Edge<NodeId> for Edge {
    fn target(&self) -> NodeId {
        self.dest
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename = "Graggle")]
pub(crate) struct GraggleData {
    nodes: Set<NodeId>,
    deleted_nodes: Set<NodeId>,
    edges: MMap<NodeId, Edge>,
    back_edges: MMap<NodeId, Edge>,

    // A partition of all the deleted nodes into weakly connected components.
    deleted_partition: Partition<NodeId>,
    // A map from pseudo-edges (the forward-pointing ones only) to the set of parts (identified by
    // their representative) that are responsible for the pseudo-edge.
    pseudo_edge_reasons: MMap<(NodeId, NodeId), NodeId>,
    // A map from "reasons" (i.e. representatives of a partition) to edges that are there because
    // of that reason.
    reason_pseudo_edges: MMap<NodeId, (NodeId, NodeId)>,
    // These are the component representatives whose components are dirty (i.e. we need to
    // recalculate the connectedness relation that they induce).
    dirty_reps: Set<NodeId>,
}

// Two Graggles compare as equal if they have the same nodes and edges (including pseudo-edges). We
// don't check the rest of the fields, as they are only there for optimization.
impl PartialEq<GraggleData> for GraggleData {
    fn eq(&self, other: &GraggleData) -> bool {
        self.nodes.eq(&other.nodes)
            && self.deleted_nodes.eq(&other.deleted_nodes)
            && self.edges.eq(&other.edges)
            && self.back_edges.eq(&other.back_edges)
    }
}

impl GraggleData {
    pub fn new() -> GraggleData {
        Default::default()
    }

    pub fn as_graggle(&'_ self) -> Graggle<'_> {
        Graggle { data: self }
    }

    pub fn all_out_edges<'b>(&'b self, node: &NodeId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.edges.get(node)
    }

    pub fn all_in_edges<'b>(&'b self, node: &NodeId) -> impl Iterator<Item = &'b Edge> + 'b {
        self.back_edges.get(node)
    }

    pub fn add_node(&mut self, id: NodeId) {
        self.nodes.insert(id);
    }

    fn has_live_edge(&self, src: &NodeId, dest: &NodeId) -> bool {
        // Construct the smallest (in the sense of Edge's order) edge that could possibly go from
        // src to dest.
        let e = Edge::new_live(*dest, PatchId::cur());
        if let Some(actual_e) = self.edges.get_from(src, &e).next() {
            // actual_e is an edge going from src to something greater than or equal to dest.
            // There's an edge from src to dest if and only if actual_e goes to dest.
            actual_e.dest == *dest && actual_e.kind == EdgeKind::Live
        } else {
            false
        }
    }

    // We just deleted the pseudo-edge from src to dest. Clean up the corresponding entries in
    // pseudo_edge_reasons and reason_pseudo_edges.
    fn remove_pseudo_edge_reasons(&mut self, src: &NodeId, dest: &NodeId) {
        let reasons = self
            .pseudo_edge_reasons
            .get(&(*src, *dest))
            .cloned()
            .collect::<Vec<_>>();
        self.pseudo_edge_reasons.remove_all(&(*src, *dest));
        for r in reasons {
            self.reason_pseudo_edges.remove(&r, &(*src, *dest));
        }
    }

    // Deletes an edge (both forward and back), but does nothing else to ensure consistency and
    // maintain invariants.
    fn internal_delete_edge(&mut self, src: &NodeId, edge: &Edge) {
        self.edges.remove(src, edge);
        let back_edge = Edge {
            dest: *src,
            // NOTE: This is not really correct: to get the right kind, we should really check
            // whether src is live. However, it still works because (assuming we resolve patch
            // dependencies correctly) every edge we delete either has two live endpoints or it is
            // a pseudo-edge (in which case it is a pseudo-edge in both directions).
            kind: edge.kind,
            patch: edge.patch,
        };
        self.back_edges.remove(&edge.dest, &back_edge);
    }

    fn internal_delete_back_edge(&mut self, dest: &NodeId, back_edge: &Edge) {
        self.back_edges.remove(dest, back_edge);
        let edge = Edge {
            dest: *dest,
            kind: back_edge.kind,
            patch: back_edge.patch,
        };
        self.edges.remove(&back_edge.dest, &edge);
    }

    pub fn unadd_node(&mut self, id: &NodeId) {
        // If we are unadding a node, it means we are unapplying the patch in which the node was
        // introduced. Since we must have already unapplied any reverse-dependencies of the patch,
        // the node must be live (it can't have been marked as deleted).
        assert!(self.nodes.contains(id));
        self.nodes.remove(id);

        // Remove all the edges that had anything to do with this node. (When unapplying a patch,
        // most of the edges would probably have already been deleted, but there might be lingering
        // pseudo-edges.)
        let out_edges = self.all_out_edges(id).cloned().collect::<Vec<_>>();
        let in_edges = self.all_in_edges(id).cloned().collect::<Vec<_>>();
        for e in out_edges {
            self.internal_delete_edge(id, &e);
            if e.kind == EdgeKind::Pseudo {
                self.remove_pseudo_edge_reasons(id, &e.dest);
            }
        }
        for e in in_edges {
            self.internal_delete_back_edge(id, &e);
            if e.kind == EdgeKind::Pseudo {
                self.remove_pseudo_edge_reasons(&e.dest, id);
            }
        }

        // Because we just unadded a node that was live, it can't have any effect on pseudo-edges,
        // so no need to update them.
    }

    /// Given a live node, marks it as deleted. That is, the node doesn't vanish; it turns into a
    /// tombstone.
    ///
    /// # Panics
    /// Panics if the node doesn't exist, or if exists but is not live.
    pub fn delete_node(&mut self, id: &NodeId) {
        assert!(self.nodes.contains(id));
        self.nodes.remove(id);
        self.deleted_nodes.insert(id.clone());
        // It's possible that deleted_partition already contains this node (if pseudo-edges weren't
        // resolved recently).
        if !self.deleted_partition.contains(id.clone()) {
            self.deleted_partition.insert(id.clone());
        }

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
        self.mark_dirty(id);
    }

    pub fn undelete_node(&mut self, id: &NodeId) {
        assert!(self.deleted_nodes.contains(id));
        self.deleted_nodes.remove(id);
        self.nodes.insert(id.clone());

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
    fn delete_opposite_edge(&mut self, src: &NodeId, edge: &Edge, edge_points_forwards: bool) {
        // This is the edge_map that points in the opposite direction as `edge`.
        let opposite_edges = if edge_points_forwards {
            &mut self.back_edges
        } else {
            &mut self.edges
        };

        if edge.kind == EdgeKind::Pseudo {
            // Pseudo-edges don't get marked as deleted, they just get removed.
            let opposite_edge = Edge::new_pseudo(*src);
            opposite_edges.remove(&edge.dest, &opposite_edge);
        } else {
            // To mark the edge as deleted, we actually remove it and then add it back in again
            // (because deleted edges appear in a different position in the map).
            let mut opposite_edge = Edge::new_live(*src, edge.patch);
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
    fn undelete_opposite_edge(&mut self, src: &NodeId, edge: &Edge, edge_points_forwards: bool) {
        // This is the edge_map that points in the opposite direction as `edge`.
        let opposite_edges = if edge_points_forwards {
            &mut self.back_edges
        } else {
            &mut self.edges
        };

        // Unlike `delete_opposite_edge`, there's no change of encountering a pseudo-edge pointing
        // from `edge.dest` to `src` (because `src` was just undeleted, and while it was deleted no
        // pseudo-edges pointed at it).
        let mut opposite_edge = Edge::new_deleted(*src, edge.patch);
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
    fn merge_components(&mut self, id1: &NodeId, id2: &NodeId) {
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
    fn delete_obsolete_reason(&mut self, reason: &NodeId) {
        let obsolete_pairs = self
            .reason_pseudo_edges
            .get(reason)
            .cloned()
            .collect::<Vec<_>>();

        for (src, dest) in obsolete_pairs {
            let e = Edge::new_pseudo(dest);
            self.pseudo_edge_reasons.remove(&(src, dest), reason);
            // If that was the last reason for the pseudo-edge, delete it.
            if self.pseudo_edge_reasons.get(&(src, dest)).next().is_none() {
                self.internal_delete_edge(&src, &e);
            }
        }
        self.reason_pseudo_edges.remove_all(reason);
    }

    // Marks the component containing `id` as dirty.
    fn mark_dirty(&mut self, id: &NodeId) {
        let rep = self.deleted_partition.representative(*id);
        self.delete_obsolete_reason(&rep);
        self.dirty_reps.insert(rep);
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, patch: PatchId) {
        let from_deleted = !self.nodes.contains(&from);
        let to_deleted = !self.nodes.contains(&to);
        assert!(!from_deleted || self.deleted_nodes.contains(&from));
        assert!(!to_deleted || self.deleted_nodes.contains(&to));

        self.edges
            .insert(from, Edge::new_real(to, to_deleted, patch));
        self.back_edges
            .insert(to, Edge::new_real(from, from_deleted, patch));

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
        // that it actually encompasses multiple connected components in the new graggle.
        let graggle = self.as_graggle();
        let graph = graggle.as_full_graph();
        let sub_graph = graph.node_filtered(|u| {
            !graggle.is_live(u) && dirty_reps.contains(&self.deleted_partition.representative(*u))
        });
        let components = sub_graph.weak_components().into_parts();

        // Remove all the messed up parts from the partition, and replace them with the correct
        // ones.
        for rep in dirty_reps {
            self.deleted_partition.remove_part(rep);
        }
        for component in &components {
            // Add everything in the current component as a new component in deleted_partition.
            let mut iter = component.iter();
            // Unwrap is ok because the components are guaranteed to be non-empty.
            let rep = iter.next().unwrap();
            self.deleted_partition.insert(*rep);
            for u in iter {
                self.deleted_partition.insert(*u);
                self.deleted_partition.merge(*rep, *u);
            }
        }

        // Add in the required pseudo-edges and fix up the partition.
        for component in components {
            self.add_component_pseudo_edges(&component);
        }
    }

    /// # Panics
    ///
    /// Panics unless `from` and `to` are nodes in this graggle. In particular, if you're planning to
    /// remove some nodes and the edge between them, you need to remove the edge first.
    pub fn unadd_edge(&mut self, from: &NodeId, to: &NodeId, patch: PatchId) {
        let from_deleted = self.deleted_nodes.contains(&from);
        let to_deleted = self.deleted_nodes.contains(&to);
        assert!(from_deleted || self.nodes.contains(&from));
        assert!(to_deleted || self.nodes.contains(&to));

        let forward_edge = Edge::new_real(*to, to_deleted, patch);
        let back_edge = Edge::new_real(*from, from_deleted, patch);
        self.edges.remove(&from, &forward_edge);
        self.back_edges.remove(&to, &back_edge);

        if from_deleted {
            self.mark_dirty(from);
        }
        if to_deleted {
            self.mark_dirty(to);
        }
    }

    // Adds all the pseudo-edges that are induced by a single connected component of deleted nodes.
    //
    // `component` must be a non-empty connected component of the deleted nodes.
    fn add_component_pseudo_edges(&mut self, component: &HashSet<NodeId>) {
        let graggle = self.as_graggle();
        let graph = graggle.as_full_graph();
        let mut neighborhood = graph.neighbor_set(component.iter());
        neighborhood.extend(component.iter().cloned());

        // Find the representative of this connected component. The unwrap is ok because
        // `component` is non-empty.
        let rep = self
            .deleted_partition
            .representative(*component.iter().next().unwrap());

        // This is the collection of all live nodes that are adjacent to a particular connected
        // component of deleted nodes. We will compute the complete connectivity relation that
        // the deleted nodes induce on these boundary nodes, and then we will add a pseudo-edge
        // for each connected pair.
        let boundary = neighborhood.iter().filter(|u| graggle.is_live(u));

        let mut pairs = Vec::new();
        for u in boundary {
            let sub_graph = graph.edge_filtered(|src, edge| {
                (src == u && component.contains(&edge.dest)) || component.contains(src)
            });
            for visit in sub_graph.dfs_from(u) {
                if let graph::dfs::Visit::Edge { dst, status, .. } = visit {
                    // Only take into account the first visit to a node. Besides being more
                    // efficient, this means we'll avoid adding self-loops.
                    if status == graph::dfs::Status::New && graggle.is_live(&dst) {
                        pairs.push((*u, dst));
                    }
                }
            }
        }
        for (src, dest) in pairs {
            // Only add a pseudo-edge if there is not already an edge present.
            if !self.has_live_edge(&src, &dest) {
                self.edges.insert(src, Edge::new_pseudo(dest));
                self.back_edges.insert(dest, Edge::new_pseudo(src));
                self.pseudo_edge_reasons.insert((src, dest), rep);
                self.reason_pseudo_edges.insert(rep, (src, dest));
            }
        }
    }

    fn is_live(&self, node: &NodeId) -> bool {
        self.nodes.contains(node)
    }

    // Brute-force compute the pseudo-edges that should start at node u.
    fn pseudo_edges(&self, u: &NodeId) -> HashSet<NodeId> {
        use graph::dfs::{Status, Visit};

        let mut ret = HashSet::new();
        // Pseudo-edges that should start at u are those that can be reached from u by ignoring
        // other pseudo-edges, and only going through deleted intermediate edges. This latter
        // property can be enforced by only traversing edges that either go from u to a deleted
        // node or else start at a deleted node.
        let graph = self.as_graggle().as_full_graph();
        let u_graph = graph.edge_filtered(|src, edge| {
            edge.kind != EdgeKind::Pseudo
                && ((src == u && !self.is_live(&edge.dest)) || !self.is_live(src))
        });
        for visit in u_graph.dfs_from(u) {
            if let Visit::Edge { dst, status, .. } = visit {
                if status == Status::New
                    && dst != *u
                    && self.is_live(&dst)
                    && !self.has_live_edge(u, &dst)
                {
                    ret.insert(dst);
                }
            }
        }
        ret
    }

    pub fn assert_consistent(&self) {
        // The live and deleted nodes should be disjoint.
        assert!(self.nodes.is_disjoint(&self.deleted_nodes));

        let node_exists = |id| self.nodes.contains(id) || self.deleted_nodes.contains(id);
        // The source and destination of every edge should exist somewhere, and they should not be
        // the same.
        // The destination should be deleted if and only if the edge kind is `Deleted`.
        // There should be a one-to-one correspondence between edges and back_edges.
        let mut seen_back_edges = HashSet::new();
        for (src, edge) in self.edges.iter() {
            assert!(node_exists(src));
            assert!(node_exists(&edge.dest));
            assert!(src != &edge.dest);
            assert_eq!(
                self.deleted_nodes.contains(&edge.dest),
                edge.kind == EdgeKind::Deleted
            );

            let back_edge = Edge {
                dest: *src,
                kind: if edge.kind == EdgeKind::Pseudo {
                    EdgeKind::Pseudo
                } else {
                    EdgeKind::from_deleted(self.deleted_nodes.contains(src))
                },
                patch: edge.patch,
            };
            assert!(self.back_edges.contains(&edge.dest, &back_edge));
            seen_back_edges.insert((edge.dest, back_edge));
        }
        // We've checked that every forward edge corresponds to a backward edge; now check that
        // every backward edge was encountered in this way.
        for (src, back_edge) in self.back_edges.iter() {
            assert!(seen_back_edges.contains(&(*src, *back_edge)));
        }

        // The deleted partition should contain all of the deleted nodes (if the pseudo-edges
        // haven't been resolved yet, it may also contain nodes that have been undeleted).
        for u in &self.deleted_nodes {
            assert!(self.deleted_partition.contains(*u));
        }

        // If the pseudo-edges are up-to-date, there are some additional checks we can do.
        if self.dirty_reps.is_empty() {
            // Everything in the deleted partition should be a deleted node.
            for u in self.deleted_partition.iter_parts().flat_map(|p| p) {
                assert!(self.deleted_nodes.contains(&u));
            }

            // Every pseudo-edge should have at least one reason.
            for (src, edge) in self.edges.iter() {
                if edge.kind == EdgeKind::Pseudo {
                    assert!(self
                        .pseudo_edge_reasons
                        .get(&(*src, edge.dest))
                        .next()
                        .is_some());
                }
            }

            // Every reason should correspond to a pseudo-edge.
            for (&(src, dest), _) in self.pseudo_edge_reasons.iter() {
                assert!(self.edges.contains(&src, &Edge::new_pseudo(dest)));
            }

            // Every reason should be a representative in the partition.
            for (reason, _) in self.reason_pseudo_edges.iter() {
                assert!(self.deleted_partition.is_rep(reason));
            }

            // Check that the pseudo-edges are correct.
            for u in &self.nodes {
                let correct_pseudo_edges = self.pseudo_edges(u);
                let actual_pseudo_edges = self
                    .all_out_edges(u)
                    .filter(|e| e.kind == EdgeKind::Pseudo)
                    .map(|e| e.dest)
                    .collect::<HashSet<_>>();
                assert_eq!(correct_pseudo_edges, actual_pseudo_edges);
            }
        }
    }
}

// This wrapping is a bit annoying. It would be simpler just to rename `GraggleData` to `Graggle` and
// then pass around `&Graggle`s. The thing is that we want to implement `Graph` for `&Graggle`, and I
// had some problems with that for some reason (can no longer remember why...). Certainly, the lack
// of ATCs/GATs means we can't implement `Graph` for `Graggle`.
/// A graggle is like a file, except that its lines are not necessarily in a linear order (rather,
/// they form a directed graph).
///
/// This is a read-only view into a graggle. It implements [`Graph`](graph::Graph), so you may
/// apply graph-based algorithms on it.
///
/// Note that lines in a graggle may be either live or deleted (nodes that are ``deleted'' are not
/// actually removed, but they are simply marked as being deleted). Some of the methods on `Graggle`
/// ignore the deleted lines, while others expose them.
//
// TODO: should explain back-edges and pseudo-edges here
#[derive(Clone, Copy, Debug)]
pub struct Graggle<'a> {
    data: &'a GraggleData,
}

impl<'a> Graggle<'a> {
    /// Returns an iterator over all live nodes of this graggle.
    pub fn nodes(self) -> impl Iterator<Item = NodeId> + 'a {
        self.data.nodes.iter().cloned()
    }

    /// Returns an iterator over all edges pointing from `node` to another live node.
    pub fn out_edges(self, node: &NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.data.edges.get(node).take_while(|e| e.not_deleted())
    }

    /// Returns an iterator over all live out-neighbors of `node`.
    pub fn out_neighbors(self, node: &NodeId) -> impl Iterator<Item = &'a NodeId> + 'a {
        self.out_edges(node).map(|e| &e.dest)
    }

    /// Returns an iterator over all live in-neighbors of `node`.
    pub fn in_neighbors(self, node: &NodeId) -> impl Iterator<Item = &'a NodeId> + 'a {
        self.in_edges(node).map(|e| &e.dest)
    }

    /// Returns an iterator over all edges pointing out of `node`, including those that point to
    /// deleted edges.
    pub fn all_out_edges(self, node: &NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.data.edges.get(node)
    }
    /// Returns an iterator over all backwards edges pointing from `node` to another live node.
    pub fn in_edges(self, node: &NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.data
            .back_edges
            .get(node)
            .take_while(|e| e.not_deleted())
    }

    /// Returns an iterator over all backwards edges pointing out of `node`, including those that
    /// point to deleted edges.
    pub fn all_in_edges(self, node: &NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.data.back_edges.get(node)
    }

    /// Returns `true` if `node` belongs to this graggle (whether it is live or deleted).
    pub fn has_node(self, node: &NodeId) -> bool {
        self.data.nodes.contains(node) || self.data.deleted_nodes.contains(node)
    }

    /// Returns `true` if `node` is live.
    ///
    /// # Panics
    ///
    /// Panics unless `node` belongs to this graggle.
    pub fn is_live(self, node: &NodeId) -> bool {
        assert!(self.has_node(node));
        self.data.nodes.contains(node)
    }

    /// Wraps `self` in [`LiveGraph`], which implements [`graph::Graph`] over the live nodes of
    /// this graggle.
    pub fn as_live_graph(self) -> LiveGraph<'a> {
        LiveGraph(self)
    }

    /// Wraps `self` in [`FullGraph`], which implements [`graph::Graph`] over all (live and
    /// deleted) nodes of this graggle.
    pub fn as_full_graph(self) -> FullGraph<'a> {
        FullGraph(self)
    }
}

impl<'a> From<&'a GraggleData> for Graggle<'a> {
    fn from(d: &'a GraggleData) -> Graggle<'a> {
        Graggle { data: d }
    }
}

/// A wrapper around [`Graggle`] implementing the [`graph::Graph`] trait.
///
/// This represents only the part of the graggle containing live nodes. To examine the entire graggle
/// (i.e. including deleted nodes), use [`FullGraph`].
pub struct LiveGraph<'a>(Graggle<'a>);

impl<'a> graph::Graph for LiveGraph<'a> {
    type Node = NodeId;
    type Edge = Edge;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = Self::Node> + 'b> {
        Box::new(self.0.data.nodes.iter().cloned())
    }

    fn out_edges<'b>(&'b self, u: &NodeId) -> Box<dyn Iterator<Item = Self::Edge> + 'b> {
        Box::new(self.0.out_edges(u).cloned())
    }

    fn in_edges<'b>(&'b self, u: &NodeId) -> Box<dyn Iterator<Item = Self::Edge> + 'b> {
        Box::new(self.0.in_edges(u).cloned())
    }
}

/// A wrapper around [`Graggle`] implementing the [`graph::Graph`] trait.
///
/// This represents only the entire graggle, even the nodes that are deleted.  To examine only the
/// live parts of the graggle, use [`LiveGraph`].
pub struct FullGraph<'a>(Graggle<'a>);

impl<'a> graph::Graph for FullGraph<'a> {
    type Node = NodeId;
    type Edge = Edge;

    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = Self::Node> + 'b> {
        Box::new(
            self.0
                .data
                .nodes
                .iter()
                .chain(self.0.data.deleted_nodes.iter())
                .cloned(),
        )
    }

    fn out_edges<'b>(&'b self, u: &NodeId) -> Box<dyn Iterator<Item = Self::Edge> + 'b> {
        Box::new(self.0.all_out_edges(u).cloned())
    }

    fn in_edges<'b>(&'b self, u: &NodeId) -> Box<dyn Iterator<Item = Self::Edge> + 'b> {
        Box::new(self.0.all_in_edges(u).cloned())
    }
}

#[cfg(test)]
#[macro_use]
pub mod tests;
