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

use super::*;
use crate::patch::Change;
use crate::{NodeId, PatchId};

use byteorder::{LittleEndian, WriteBytesExt};
use proptest::collection::hash_set;
use proptest::prelude::*;
use proptest::sample::subsequence;
use std::sync::atomic::{AtomicUsize, Ordering};

#[doc(hidden)]
#[macro_export]
macro_rules! graggle {
    (
        $(live : $( $live:literal ),*)?
        $(deleted : $( $deleted:literal ),*)?
        $(edges : $( $src:literal - $dest:literal ),*)?
    ) => {
        {
            let mut d = $crate::storage::graggle::GraggleData::new();
            $($(
                d.add_node(NodeId::cur($live));
            )*)*
            $($(
                d.add_node(NodeId::cur($deleted));
                d.delete_node(&NodeId::cur($deleted));
            )*)*
            $($(
                d.add_edge(NodeId::cur($src), NodeId::cur($dest), $crate::PatchId::cur());
            )*)*
            d
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChangesWithId {
    pub changes: Vec<Change>,
    pub id: PatchId,
}

#[doc(hidden)]
#[macro_export]
macro_rules! changes {
    (
        $(delete : $( $delete_node:literal ),*)?
        $(nodes : $( $add_node:literal ),*)?
        $(edges : $( $src:literal - $dest:literal ),*)?
    ) => {{
        $crate::storage::graggle::tests::ChangesWithId {
            changes: vec![
                $($(
                    Change::DeleteNode { id: NodeId::cur($delete_node) },
                )*)*
                $($(
                    Change::NewNode { id: NodeId::cur($add_node), contents: vec![] },
                )*)*
                $($(
                    Change::NewEdge { src: NodeId::cur($src), dest: NodeId::cur($dest) },
                )*)*
            ],
            id: PatchId::cur(),
        }
    }}
}

macro_rules! assert_pseudoedges {
    ($d:expr; $( $psrc:literal - $pdest:literal ),*) => {
        {
            $d.assert_consistent();
            $d.resolve_pseudo_edges();
            $d.assert_consistent();

            // Squash the warning that would appear if no pseudo-edges are expected
            #[allow(unused_mut)]
            let mut expected = HashSet::new();
            $( expected.insert(($psrc, $pdest)); )*

            assert_eq!(expected, $d.pseudoedges());
        }
    }
}

trait GraggleExt {
    fn has_pseudoedge(&self, i: u64, j: u64) -> bool;
    fn pseudoedges(&self) -> HashSet<(u64, u64)>;
}

impl GraggleExt for GraggleData {
    fn has_pseudoedge(&self, i: u64, j: u64) -> bool {
        let src = NodeId::cur(i);
        let edge = Edge::new_pseudo(NodeId::cur(j));
        self.edges.contains(&src, &edge)
    }

    fn pseudoedges(&self) -> HashSet<(u64, u64)> {
        self.edges
            .iter()
            .filter(|(_, e)| e.kind == EdgeKind::Pseudo)
            .map(|(src, e)| (src.node, e.dest.node))
            .collect::<HashSet<_>>()
    }
}

#[test]
fn delete_middle() {
    let mut d = graggle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2
    );
    assert_pseudoedges!(d; 0-2);
}

// If there is already an edge, there shouldn't be a pseudoedge added.
#[test]
fn delete_middle_existing() {
    let mut d = graggle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2, 0-2
    );
    assert_pseudoedges!(d; );
}

#[test]
fn delete_long_middle() {
    let mut d = graggle!(
        live: 0, 5
        deleted: 1, 2, 3, 4
        edges: 0-1, 1-2, 2-3, 3-4, 4-5
    );
    assert_pseudoedges!(d; 0-5);

    d.unadd_edge(&NodeId::cur(2), &NodeId::cur(3), PatchId::cur());
    assert_pseudoedges!(d; );

    d.add_edge(NodeId::cur(2), NodeId::cur(3), PatchId::cur());
    assert_pseudoedges!(d; 0-5);
    d.undelete_node(&NodeId::cur(3));
    assert_pseudoedges!(d; 0-3, 3-5);
    d.undelete_node(&NodeId::cur(1));
    d.undelete_node(&NodeId::cur(2));
    assert_pseudoedges!(d; 3-5);
    d.undelete_node(&NodeId::cur(4));
    assert_pseudoedges!(d; );
}

// Adding a node next to a deleted node might cause a pseudo-edge.
#[test]
fn add_next_to_deleted() {
    let mut d = graggle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2
    );
    d.add_node(NodeId::cur(3));
    d.add_edge(NodeId::cur(1), NodeId::cur(3), PatchId::cur());
    assert_pseudoedges!(d; 0-2, 0-3);
}

// All of the non-deleted edges have paths from one to the other, but none of those paths goes
// through the deleted part, so there should be no pseudo-edges.
#[test]
fn boundary_vs_interior() {
    let mut d = graggle!(
        live: 0, 1, 2
        deleted: 3
        edges: 0-1, 1-2, 2-0, 0-3, 1-3, 2-3
    );
    assert_pseudoedges!(d; );
}

// Adding edges back from the deleted component means we get the pseudo-edges after all.
#[test]
fn boundary_vs_interior_connected() {
    let mut d = graggle!(
        live: 0, 1, 2
        deleted: 3
        edges: 0-1, 1-2, 2-0, 0-3, 1-3, 2-3, 3-0, 3-1, 3-2
    );
    assert_pseudoedges!(d; 0-2, 2-1, 1-0);
}

// There are two reasons for the pseudo-edge between 0 and 3.
#[test]
fn two_reasons() {
    let mut d = graggle!(
        live: 0, 3
        deleted: 1, 2
        edges: 0-1, 1-3, 0-2, 2-3
    );
    assert_pseudoedges!(d; 0-3);

    // If we get rid of one reason, the pseudo-edge should still be there.
    d.undelete_node(&NodeId::cur(1));
    assert_pseudoedges!(d; 0-3);

    d.undelete_node(&NodeId::cur(2));
    assert_pseudoedges!(d; );
}

// It's legal for two different patches to add the same edge.
#[test]
fn duplicate_edge() {
    let d = graggle!(
        live: 0, 1
    );
    let mut ch1 = changes!(
        edges: 0-1
    );
    let mut ch2 = changes!(
        edges: 0-1
    );

    // The changes! macro defaults to setting all ids to PatchId::cur, which isn't really correct
    // but it doesn't cause an error unless there are (like here) duplicate edges.
    ch1.id.data[0] = 1;
    ch2.id.data[0] = 2;

    check_graggle_and_changes(d, &[ch1, ch2]);
}

// When generating graggles, we could in principle put in as many as n^2 edges, but that's way
// too many to be realistic (a realistic value would be around 2). So we allow only up to
// n*MAX_AVG_DEGREE.
const MAX_AVG_DEGREE: usize = 5;

fn fake_patch_id(id: usize) -> PatchId {
    let mut ret = PatchId::cur();
    (&mut ret.data[..])
        .write_u64::<LittleEndian>(id as u64)
        .unwrap();
    ret
}

// Make a graggle like 0 -> 1 -> 2, and delete node 1.
#[test]
fn append_and_delete() {
    let d = graggle!(
        live: 0, 1
        edges: 0-1
    );
    let ch = changes!(
        delete: 1
        nodes: 2
        edges: 1-2
    );

    // Manually check the presence of a pseudo-edge from 0 to 2.
    let mut clone = d.clone();
    apply_changes(&mut clone, &ch);
    clone.resolve_pseudo_edges();
    assert_pseudoedges!(clone; 0-2);

    // Now run the exhaustive checks.
    check_graggle_and_changes(d, &[ch]);
}

fn check_graggle_and_changes(d: GraggleData, chs: &[ChangesWithId]) {
    let orig = d.clone();

    // Apply all the changes one-by-one. At each step, check that reversing the change
    // produces the previous graggle.
    let mut cur = d.clone();

    for ch in chs {
        let mut next = cur.clone();
        apply_changes(&mut next, ch);
        next.assert_consistent();
        next.resolve_pseudo_edges();
        next.assert_consistent();

        let mut unapplied = next.clone();
        unapply_changes(&mut unapplied, ch);
        unapplied.assert_consistent();
        unapplied.resolve_pseudo_edges();
        unapplied.assert_consistent();
        assert_eq!(cur, unapplied);

        cur = next;
    }

    // Try applying *all* of the changes, and then resolving. It shouldn't affect the
    // answer.
    let mut all_at_once = d.clone();
    for ch in chs {
        apply_changes(&mut all_at_once, ch);
    }
    all_at_once.assert_consistent();
    all_at_once.resolve_pseudo_edges();
    all_at_once.assert_consistent();
    assert_eq!(cur, all_at_once);

    // Now unapply them all and make sure it agrees with the original.
    for ch in chs.iter().rev() {
        unapply_changes(&mut all_at_once, ch);
    }
    all_at_once.assert_consistent();
    all_at_once.resolve_pseudo_edges();
    all_at_once.assert_consistent();
    assert_eq!(orig, all_at_once);

    // Now we do the last thing again, but without resolving pseudo-edges after applying all the
    // patches.
    for ch in chs {
        apply_changes(&mut all_at_once, ch);
    }
    for ch in chs.iter().rev() {
        unapply_changes(&mut all_at_once, ch);
    }
    all_at_once.assert_consistent();
    all_at_once.resolve_pseudo_edges();
    assert_eq!(orig, all_at_once);

    // Now we do the same thing, but in the other order: unapply them all and then apply them all,
    // without resolving pseudo-edges in between.
    let mut all_at_once = cur.clone();
    for ch in chs.iter().rev() {
        unapply_changes(&mut all_at_once, ch);
    }
    for ch in chs {
        apply_changes(&mut all_at_once, ch);
    }
    all_at_once.assert_consistent();
    all_at_once.resolve_pseudo_edges();
    assert_eq!(cur, all_at_once);
}

// This example was found by proptest.
#[test]
fn two_changes() {
    let g = graggle!(
        live: 0
    );

    let ch1 = changes!(
        nodes: 1
        edges: 1-0
    );
    let ch2 = changes!(
        delete: 1
        nodes: 2
        edges: 2-0, 0-2, 1-2
    );
    check_graggle_and_changes(g, &[ch1, ch2]);
}

// Create a graggle of three nodes by making the outer two first, and then adding the middle one.
#[test]
fn add_middle() {
    let d = graggle!(
        live: 0, 2
    );

    let changes1 = changes!(
        nodes: 1
        edges: 0-1, 1-2
    );

    let changes2 = changes!(
        delete: 1
    );

    check_graggle_and_changes(d, &[changes1, changes2]);
}

// This exercises the code for rebuilding the deleted partition.
#[test]
fn reconstruct_partition() {
    let d = graggle!(
        live: 0, 1, 2
        edges: 0-1
    );
    let ch = changes! {
        delete: 0, 1, 2
        nodes: 10, 11
        edges: 11-0, 10-2, 0-10, 2-11
    };
    check_graggle_and_changes(d, &[ch]);
}

// Checks that when we delete a pseudo-edge reason, we don't delete the pseudo-edge as long as
// there is another reason.
#[test]
fn double_reason() {
    let d = graggle!(
        live: 0, 1, 2, 3
        edges: 0-1, 1-0, 2-0, 3-0, 3-1
    );
    let ch1 = changes!(
        delete: 1, 2, 3
        nodes: 10, 11
        edges: 11-10, 10-0, 10-3, 10-1, 1-10, 0-11
    );
    let ch2 = changes!(
        delete: 11
    );

    check_graggle_and_changes(d, &[ch1, ch2]);
}

// This was generated by proptest. It has lots of edges, so there are lots of opportunities to
// hit an edge-case in pseudo-edge generation.
#[test]
fn lots_of_edges() {
    let d = graggle!(live: 0, 1, 2, 3);
    let ch1 = changes!(
        delete: 0, 1, 3
        nodes: 10, 11, 12, 13
        edges: 12-10, 12-11, 12-2, 13-3, 12-1, 11-3, 11-1, 13-0, 10-2, 11-0, 10-1, 10-0,
               3-13, 1-10, 2-10, 0-11, 3-12
    );
    let ch2 = changes!(
        delete: 10
    );

    check_graggle_and_changes(d, &[ch1, ch2]);
}

#[test]
fn delete_and_undelete() {
    let d = graggle!(live: 0);
    let ch = changes!(delete: 0);
    check_graggle_and_changes(d, &[ch]);
}

prop_compose! {
    // Creates an arbitrary graggle with no deleted nodes.
    [pub(crate)] fn arb_live_graggle(max_nodes: usize)
                     (num_nodes in 1..max_nodes)
                     (edges in hash_set((0..num_nodes, 0..num_nodes), 0..(num_nodes * MAX_AVG_DEGREE)),
                      num_nodes in Just(num_nodes))
                     -> GraggleData
    {
        let mut ret = GraggleData::new();
        for i in 0..num_nodes {
            ret.nodes.insert(NodeId::cur(i as u64));
        }
        for (u, v) in edges {
            if u != v {
                let u = NodeId::cur(u as u64);
                let v = NodeId::cur(v as u64);
                ret.edges.insert(u, Edge::new_live(v, PatchId::cur()));
                ret.back_edges.insert(v, Edge::new_live(u, PatchId::cur()));
            }
        }
        ret
    }
}

// When we create different `Changes`, we need to give each one a unique PatchId. We achieve
// this by simply incrementing a counter. We start from 1, because by default the graggles that
// we create use the id 0.
static CUR_ID: AtomicUsize = AtomicUsize::new(1);

// Create arbitrary patches on top of graggles. Basically, an arbitrary patch consists of an
// arbitrary subset of nodes to delete, and an arbitrary set of nodes to add, with arbitrary
// edges between the new nodes, and also between the new nodes and the old ones.
fn arb_changes<'a>(graggle: &'a GraggleData, size: usize) -> BoxedStrategy<ChangesWithId> {
    fn make_changes(
        old_ids: Vec<NodeId>,
        nodes_to_delete: Vec<NodeId>,
        num_to_add: usize,
        new_new_edges: HashSet<(usize, usize)>,
        new_old_edges: HashSet<(usize, usize)>,
        old_new_edges: HashSet<(usize, usize)>,
    ) -> ChangesWithId {
        let patch_id_int = CUR_ID.fetch_add(1, Ordering::SeqCst);
        let patch_id = fake_patch_id(patch_id_int);

        let new_ids = (0..num_to_add)
            .map(|i| NodeId {
                patch: patch_id,
                node: i as u64,
            })
            .collect::<Vec<_>>();

        let deletions = nodes_to_delete
            .iter()
            .map(|u| Change::DeleteNode { id: *u });

        let insertions = new_ids.iter().map(|u| Change::NewNode {
            id: *u,
            contents: vec![],
        });

        let edges = new_new_edges
            .into_iter()
            .map(|(i, j)| (new_ids[i], new_ids[j]))
            .chain(
                new_old_edges
                    .into_iter()
                    .map(|(i, j)| (new_ids[i], old_ids[j])),
            )
            .chain(
                old_new_edges
                    .into_iter()
                    .map(|(i, j)| (old_ids[i], new_ids[j])),
            )
            .filter(|(u, v)| u != v);
        let edges = edges.map(|(u, v)| Change::NewEdge { src: u, dest: v });

        let changes = deletions.chain(insertions).chain(edges).collect::<Vec<_>>();
        ChangesWithId {
            changes,
            id: patch_id,
        }
    }

    let old_ids = graggle.nodes.iter().cloned().collect::<Vec<_>>();
    let num_to_add = 1..size;

    // Strategy returning a tuple
    // (nodes_to_delete, num_to_add, new_new_edges, new_old_edges, old_new_edges)
    let old = old_ids.clone();
    let changes = num_to_add.prop_flat_map(move |n| {
        (
            subsequence(old.clone(), 0..old.len()),
            Just(n),
            hash_set((0..n, 0..n), 0..(MAX_AVG_DEGREE * n)),
            hash_set((0..n, 0..old.len()), 0..(MAX_AVG_DEGREE * n.min(old.len()))),
            hash_set((0..old.len(), 0..n), 0..(MAX_AVG_DEGREE * n.min(old.len()))),
        )
    });
    changes
        .prop_map(move |(del, n, nn, no, on)| make_changes(old_ids.clone(), del, n, nn, no, on))
        .boxed()
}

// Creates an arbitrary graggle and a change that can be applied to it.
fn arb_graggle_and_change(
    initial_size: usize,
    change_size: usize,
) -> BoxedStrategy<(GraggleData, ChangesWithId)> {
    let graggle = arb_live_graggle(initial_size);
    graggle
        .prop_flat_map(move |d| {
            let ch = arb_changes(&d, change_size);
            (Just(d), ch)
        })
        .boxed()
}

proptest! {
    #[test]
    fn live_graggles_consistent(ref d in arb_live_graggle(20)) {
        d.assert_consistent();
    }
}

// These two functions are basically copy&paste from `Storage`. TODO: consider refactoring
fn apply_changes(graggle: &mut GraggleData, changes: &ChangesWithId) {
    for ch in &changes.changes {
        match *ch {
            Change::NewNode { ref id, .. } => graggle.add_node(id.clone()),
            Change::DeleteNode { ref id } => graggle.delete_node(&id),
            Change::NewEdge { ref src, ref dest } => {
                graggle.add_edge(src.clone(), dest.clone(), changes.id)
            }
        }
    }
}

fn unapply_changes(graggle: &mut GraggleData, changes: &ChangesWithId) {
    for ch in &changes.changes {
        match *ch {
            Change::DeleteNode { ref id } => graggle.undelete_node(id),
            Change::NewEdge { ref src, ref dest } => graggle.unadd_edge(src, dest, changes.id),
            Change::NewNode { .. } => {}
        }
    }
    for ch in &changes.changes {
        if let Change::NewNode { ref id, .. } = *ch {
            graggle.unadd_node(id);
        }
    }
}

proptest! {
    #[test]
    fn graggle_then_change((ref d, ref ch) in arb_graggle_and_change(20, 10)) {
        let mut d = d.clone();
        d.assert_consistent();

        apply_changes(&mut d, ch);
        d.assert_consistent();

        d.resolve_pseudo_edges();
        d.assert_consistent();

        unapply_changes(&mut d, ch);
        d.assert_consistent();

        d.resolve_pseudo_edges();
        d.assert_consistent();
    }
}

// Creates an arbitrary graggle and a sequence of changes, which can be applied to the graggle
// one-by-one.
fn arb_graggle_and_change_seq(
    initial_size: usize,
    change_size: usize,
    num_changes: usize,
) -> BoxedStrategy<(GraggleData, Vec<ChangesWithId>)> {
    fn recurse(
        orig: GraggleData,
        change_size: usize,
        num_changes: usize,
        cur: GraggleData,
        changes: Vec<ChangesWithId>,
    ) -> BoxedStrategy<(GraggleData, Vec<ChangesWithId>)> {
        if num_changes == 0 {
            Just((orig, changes)).boxed()
        } else {
            let next_change = arb_changes(&cur, change_size);
            (Just(orig), Just(cur), Just(changes), next_change)
                .prop_flat_map(move |(orig, mut cur, mut changes, ch)| {
                    apply_changes(&mut cur, &ch);
                    changes.push(ch);
                    recurse(orig, change_size, num_changes - 1, cur, changes)
                })
                .boxed()
        }
    }
    let graggle = arb_live_graggle(initial_size);
    let num_changes = 1..(num_changes + 1);
    (graggle, num_changes)
        .prop_flat_map(move |(d, n)| recurse(d.clone(), change_size, n, d, vec![]))
        .boxed()
}

proptest! {
    // This takes a really long time to shrink, so cap it at 30 seconds.
    #![proptest_config(ProptestConfig {
        max_shrink_time: 30000,
        .. ProptestConfig::default()
    })]

    #[test]
    fn graggle_and_change_seq((ref d, ref chs) in arb_graggle_and_change_seq(10, 5, 3)) {
        // Apply all the changes one-by-one. At each step, check that reversing the change
        // produces the previous graggle.
        let mut cur = d.clone();
        for ch in chs {
            let mut next = cur.clone();
            apply_changes(&mut next, ch);
            next.resolve_pseudo_edges();
            next.assert_consistent();

            let mut unapplied = next.clone();
            unapply_changes(&mut unapplied, ch);
            unapplied.resolve_pseudo_edges();
            unapplied.assert_consistent();
            assert_eq!(cur, unapplied);

            cur = next;
        }

        // Try applying *all* of the changes, and then resolving. It shouldn't affect the
        // answer.
        let mut all_at_once = d.clone();
        for ch in chs {
            apply_changes(&mut all_at_once, ch);
        }
        all_at_once.assert_consistent();
        all_at_once.resolve_pseudo_edges();
        all_at_once.assert_consistent();
        assert_eq!(cur, all_at_once);
    }
}
