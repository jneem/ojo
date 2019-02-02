use super::*;
use crate::patch::{Change, Changes};
use crate::{NodeId, PatchId};

use byteorder::{LittleEndian, WriteBytesExt};
use proptest::collection::hash_set;
use proptest::prelude::*;
use proptest::sample::subsequence;
use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! digle {
    (
        $(live : $( $live:literal ),*)?
        $(deleted : $( $deleted:literal ),*)?
        $(edges : $( $src:literal - $dest:literal ),*)?
    ) => {
        {
            let mut d = DigleData::new();
            $($(
                d.add_node(NodeId::cur($live));
            )*)*
            $($(
                d.add_node(NodeId::cur($deleted));
                d.delete_node(&NodeId::cur($deleted));
            )*)*
            $($(
                d.add_edge(NodeId::cur($src), NodeId::cur($dest));
            )*)*
            d
        }
    }
}

macro_rules! changes {
    (
        $(delete : $( $delete_node:literal ),*)?
        $(nodes : $( $add_node:literal ),*)?
        $(edges : $( $src:literal - $dest:literal ),*)?
    ) => {
        Changes {
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
        }
    }
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

trait DigleExt {
    fn has_pseudoedge(&self, i: u64, j: u64) -> bool;
    fn pseudoedges(&self) -> HashSet<(u64, u64)>;
}

impl DigleExt for DigleData {
    fn has_pseudoedge(&self, i: u64, j: u64) -> bool {
        let src = NodeId::cur(i);
        let edge = Edge {
            dest: NodeId::cur(j),
            kind: EdgeKind::Pseudo,
        };
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
    let mut d = digle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2
    );
    assert_pseudoedges!(d; 0-2);
}

// If there is already an edge, there shouldn't be a pseudoedge added.
#[test]
fn delete_middle_existing() {
    let mut d = digle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2, 0-2
    );
    assert_pseudoedges!(d; );
}

#[test]
fn delete_long_middle() {
    let mut d = digle!(
        live: 0, 5
        deleted: 1, 2, 3, 4
        edges: 0-1, 1-2, 2-3, 3-4, 4-5
    );
    assert_pseudoedges!(d; 0-5);

    d.unadd_edge(&NodeId::cur(2), &NodeId::cur(3));
    assert_pseudoedges!(d; );

    d.add_edge(NodeId::cur(2), NodeId::cur(3));
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
    let mut d = digle!(
        live: 0, 2
        deleted: 1
        edges: 0-1, 1-2
    );
    d.add_node(NodeId::cur(3));
    d.add_edge(NodeId::cur(1), NodeId::cur(3));
    assert_pseudoedges!(d; 0-2, 0-3);
}

// All of the non-deleted edges have paths from one to the other, but none of those paths goes
// through the deleted part, so there should be no pseudo-edges.
#[test]
fn boundary_vs_interior() {
    let mut d = digle!(
        live: 0, 1, 2
        deleted: 3
        edges: 0-1, 1-2, 2-0, 0-3, 1-3, 2-3
    );
    assert_pseudoedges!(d; );
}

// Adding edges back from the deleted component means we get the pseudo-edges after all.
#[test]
fn boundary_vs_interior_connected() {
    let mut d = digle!(
        live: 0, 1, 2
        deleted: 3
        edges: 0-1, 1-2, 2-0, 0-3, 1-3, 2-3, 3-0, 3-1, 3-2
    );
    assert_pseudoedges!(d; 0-2, 2-1, 1-0);
}

// There are two reasons for the pseudo-edge between 0 and 3.
#[test]
fn two_reasons() {
    let mut d = digle!(
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

// When generating digles, we could in principle put in as many as n^2 edges, but that's way
// too many to be realistic (a realistic value would be around 2). So we allow only up to
// n*MAX_AVG_DEGREE.
const MAX_AVG_DEGREE: usize = 5;

// Given a string like "0-3, 1-2, 3-4, 2-3", creates a digle with those edges.
pub(crate) fn make_digle(s: &str) -> DigleData {
    let pairs = s
        .split(',')
        .map(|elt| {
            let dash_idx = elt.find('-').unwrap();
            let u: usize = elt[..dash_idx].trim().parse().unwrap();
            let v: usize = elt[(dash_idx + 1)..].trim().parse().unwrap();
            (u, v)
        })
        .collect::<Vec<_>>();
    let max_elt = pairs
        .iter()
        .flat_map(|pair| vec![pair.0, pair.1].into_iter())
        .max()
        .unwrap();

    let mut ret = DigleData::new();
    for i in 0..=max_elt {
        ret.add_node(NodeId::cur(i as u64));
    }
    for (u, v) in pairs {
        ret.add_edge(NodeId::cur(u as u64), NodeId::cur(v as u64));
    }
    ret
}

fn fake_patch_id(id: usize) -> PatchId {
    let mut ret = PatchId::cur();
    (&mut ret.data[..])
        .write_u64::<LittleEndian>(id as u64)
        .unwrap();
    ret
}

// Make a digle like 0 -> 1 -> 2, and delete node 1.
#[test]
fn append_and_delete() {
    let patch_id = fake_patch_id(1);
    let node_id = NodeId {
        patch: patch_id,
        node: 0,
    };
    let mut d = make_digle("0-1");

    let changes = Changes {
        changes: vec![
            Change::DeleteNode { id: NodeId::cur(1) },
            Change::NewNode {
                id: node_id,
                contents: vec![],
            },
            Change::NewEdge {
                src: NodeId::cur(1),
                dest: node_id,
            },
        ],
    };
    apply_changes(&mut d, &changes);
    d.resolve_pseudo_edges();
    d.assert_consistent();
    assert!(d.edges.contains(
        &NodeId::cur(0),
        &Edge {
            dest: node_id,
            kind: EdgeKind::Pseudo
        }
    ));
    unapply_changes(&mut d, &changes);
    d.resolve_pseudo_edges();
    d.assert_consistent();
}

fn check_digle_and_changes(d: DigleData, chs: &[Changes]) {
    let orig = d.clone();

    // Apply all the changes one-by-one. At each step, check that reversing the change
    // produces the previous digle.
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
}

// This example was found by proptest.
#[test]
fn two_changes() {
    let node0 = NodeId::cur(0);
    let patch1 = fake_patch_id(1);
    let node1 = NodeId {
        patch: patch1,
        node: 0,
    };
    let patch2 = fake_patch_id(2);
    let node2 = NodeId {
        patch: patch2,
        node: 0,
    };
    let mut d = DigleData::new();
    d.add_node(node0);

    let changes1 = Changes {
        changes: vec![
            Change::NewNode {
                id: node1,
                contents: vec![],
            },
            Change::NewEdge {
                src: node1,
                dest: node0,
            },
        ],
    };

    let changes2 = Changes {
        changes: vec![
            Change::DeleteNode { id: node1 },
            Change::NewNode {
                id: node2,
                contents: vec![],
            },
            Change::NewEdge {
                src: node2,
                dest: node0,
            },
            Change::NewEdge {
                src: node0,
                dest: node2,
            },
            Change::NewEdge {
                src: node1,
                dest: node2,
            },
        ],
    };

    check_digle_and_changes(d, &[changes1, changes2]);
}

// Create a digle of three nodes by making the outer two first, and then adding the middle one.
#[test]
fn add_middle() {
    let node00 = NodeId::cur(0);
    let node01 = NodeId::cur(1);
    let patch1 = fake_patch_id(1);
    let node1 = NodeId {
        patch: patch1,
        node: 0,
    };
    let mut d = DigleData::new();
    d.add_node(node00);
    d.add_node(node01);

    let changes1 = Changes {
        changes: vec![
            Change::NewNode {
                id: node1,
                contents: vec![],
            },
            Change::NewEdge {
                src: node1,
                dest: node01,
            },
            Change::NewEdge {
                src: node00,
                dest: node1,
            },
        ],
    };

    let changes2 = Changes {
        changes: vec![Change::DeleteNode { id: node1 }],
    };

    check_digle_and_changes(d, &[changes1, changes2]);
}

// This exercises the code for rebuilding the deleted partition.
#[test]
fn reconstruct_partition() {
    let mut d = make_digle("0-1");
    d.add_node(NodeId::cur(2));

    let patch = fake_patch_id(1);
    let node0 = NodeId {
        patch: patch,
        node: 0,
    };
    let node1 = NodeId {
        patch: patch,
        node: 1,
    };

    let changes = Changes {
        changes: vec![
            Change::DeleteNode { id: NodeId::cur(0) },
            Change::DeleteNode { id: NodeId::cur(1) },
            Change::DeleteNode { id: NodeId::cur(2) },
            Change::NewNode {
                id: node0,
                contents: vec![],
            },
            Change::NewNode {
                id: node1,
                contents: vec![],
            },
            Change::NewEdge {
                src: node1,
                dest: NodeId::cur(0),
            },
            Change::NewEdge {
                src: node0,
                dest: NodeId::cur(2),
            },
            Change::NewEdge {
                src: NodeId::cur(0),
                dest: node0,
            },
            Change::NewEdge {
                src: NodeId::cur(2),
                dest: node1,
            },
        ],
    };

    check_digle_and_changes(d, &[changes]);
}

// Checks that when we delete a pseudo-edge reason, we don't delete the pseudo-edge as long as
// there is another reason.
#[test]
fn double_reason() {
    let d = make_digle("0-1, 1-0, 2-0, 3-0, 3-1");

    let patch1 = fake_patch_id(1);
    let node10 = NodeId {
        patch: patch1,
        node: 0,
    };
    let node11 = NodeId {
        patch: patch1,
        node: 1,
    };

    let changes1 = Changes {
        changes: vec![
            Change::DeleteNode { id: NodeId::cur(1) },
            Change::DeleteNode { id: NodeId::cur(2) },
            Change::DeleteNode { id: NodeId::cur(3) },
            Change::NewNode {
                id: node10,
                contents: vec![],
            },
            Change::NewNode {
                id: node11,
                contents: vec![],
            },
            Change::NewEdge {
                src: node11,
                dest: node10,
            },
            Change::NewEdge {
                src: node10,
                dest: NodeId::cur(0),
            },
            Change::NewEdge {
                src: node10,
                dest: NodeId::cur(3),
            },
            Change::NewEdge {
                src: node10,
                dest: NodeId::cur(1),
            },
            Change::NewEdge {
                src: NodeId::cur(1),
                dest: node10,
            },
            Change::NewEdge {
                src: NodeId::cur(0),
                dest: node11,
            },
        ],
    };

    let changes2 = Changes {
        changes: vec![Change::DeleteNode { id: node11 }],
    };

    check_digle_and_changes(d, &[changes1, changes2]);
}

// This was generated by proptest. It has lots of edges, so there are lots of opportunities to
// hit an edge-case in pseudo-edge generation.
#[test]
fn lots_of_edges() {
    let node00 = NodeId::cur(0);
    let node01 = NodeId::cur(1);
    let node02 = NodeId::cur(2);
    let node03 = NodeId::cur(3);
    let patch1 = fake_patch_id(1);
    let node10 = NodeId {
        patch: patch1,
        node: 0,
    };
    let node11 = NodeId {
        patch: patch1,
        node: 1,
    };
    let node12 = NodeId {
        patch: patch1,
        node: 2,
    };
    let node13 = NodeId {
        patch: patch1,
        node: 3,
    };

    let mut d = DigleData::new();
    d.add_node(node00);
    d.add_node(node01);
    d.add_node(node02);
    d.add_node(node03);

    let changes1 = Changes {
        changes: vec![
            Change::DeleteNode { id: node00 },
            Change::DeleteNode { id: node01 },
            Change::DeleteNode { id: node03 },
            Change::NewNode {
                id: node10,
                contents: vec![],
            },
            Change::NewNode {
                id: node11,
                contents: vec![],
            },
            Change::NewNode {
                id: node12,
                contents: vec![],
            },
            Change::NewNode {
                id: node13,
                contents: vec![],
            },
            Change::NewEdge {
                src: node12,
                dest: node10,
            },
            Change::NewEdge {
                src: node12,
                dest: node11,
            },
            Change::NewEdge {
                src: node12,
                dest: node02,
            },
            Change::NewEdge {
                src: node13,
                dest: node03,
            },
            Change::NewEdge {
                src: node12,
                dest: node01,
            },
            Change::NewEdge {
                src: node11,
                dest: node03,
            },
            Change::NewEdge {
                src: node11,
                dest: node01,
            },
            Change::NewEdge {
                src: node13,
                dest: node00,
            },
            Change::NewEdge {
                src: node10,
                dest: node02,
            },
            Change::NewEdge {
                src: node11,
                dest: node00,
            },
            Change::NewEdge {
                src: node10,
                dest: node01,
            },
            Change::NewEdge {
                src: node10,
                dest: node00,
            },
            Change::NewEdge {
                src: node03,
                dest: node13,
            },
            Change::NewEdge {
                src: node01,
                dest: node10,
            },
            Change::NewEdge {
                src: node02,
                dest: node10,
            },
            Change::NewEdge {
                src: node00,
                dest: node11,
            },
            Change::NewEdge {
                src: node03,
                dest: node12,
            },
        ],
    };

    let changes2 = Changes {
        changes: vec![Change::DeleteNode { id: node10 }],
    };

    check_digle_and_changes(d, &[changes1, changes2]);
}

#[test]
fn delete_and_undelete() {
    // TODO: this exposes a (former) bug that wasn't caught by check_digle_and_changes because it
    // only surfaced when we *avoided* resolving pseudo-edges.
    //
    // Modify check_digle_and_changes to exercise the lazily-resolving pseudo-edges also.
    let mut d = digle!(live: 0);
    let ch = changes!(delete: 0);
    apply_changes(&mut d, &ch);
    d.resolve_pseudo_edges();
    d.assert_consistent();
    unapply_changes(&mut d, &ch);
    d.assert_consistent();
    apply_changes(&mut d, &ch);
    d.assert_consistent();
}

prop_compose! {
    // Creates an arbitrary digle with no deleted nodes.
    [pub(crate)] fn arb_live_digle(max_nodes: usize)
                     (num_nodes in 1..max_nodes)
                     (edges in hash_set((0..num_nodes, 0..num_nodes), 0..(num_nodes * MAX_AVG_DEGREE)),
                      num_nodes in Just(num_nodes))
                     -> DigleData
    {
        let mut ret = DigleData::new();
        for i in 0..num_nodes {
            ret.nodes.insert(NodeId::cur(i as u64));
        }
        for (u, v) in edges {
            if u != v {
                let u = NodeId::cur(u as u64);
                let v = NodeId::cur(v as u64);
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
        old_ids: Vec<NodeId>,
        nodes_to_delete: Vec<NodeId>,
        num_to_add: usize,
        new_new_edges: HashSet<(usize, usize)>,
        new_old_edges: HashSet<(usize, usize)>,
        old_new_edges: HashSet<(usize, usize)>,
    ) -> Changes {
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
        Changes { changes }
    }

    let old_ids = digle.nodes.iter().cloned().collect::<Vec<_>>();
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

// Creates an arbitrary digle and a change that can be applied to it.
fn arb_digle_and_change(
    initial_size: usize,
    change_size: usize,
) -> BoxedStrategy<(DigleData, Changes)> {
    let digle = arb_live_digle(initial_size);
    digle
        .prop_flat_map(move |d| {
            let ch = arb_changes(&d, change_size);
            (Just(d), ch)
        })
        .boxed()
}

proptest! {
    #[test]
    fn live_digles_consistent(ref d in arb_live_digle(20)) {
        d.assert_consistent();
    }
}

// These two functions are basically copy&paste from `Storage`. TODO: consider refactoring
fn apply_changes(digle: &mut DigleData, changes: &Changes) {
    for ch in &changes.changes {
        match *ch {
            Change::NewNode { ref id, .. } => digle.add_node(id.clone()),
            Change::DeleteNode { ref id } => digle.delete_node(&id),
            Change::NewEdge { ref src, ref dest } => digle.add_edge(src.clone(), dest.clone()),
        }
    }
}

fn unapply_changes(digle: &mut DigleData, changes: &Changes) {
    for ch in &changes.changes {
        match *ch {
            Change::DeleteNode { ref id } => digle.undelete_node(id),
            Change::NewEdge { ref src, ref dest } => digle.unadd_edge(src, dest),
            Change::NewNode { .. } => {}
        }
    }
    for ch in &changes.changes {
        if let Change::NewNode { ref id, .. } = *ch {
            digle.unadd_node(id);
        }
    }
}

proptest! {
    #[test]
    fn digle_then_change((ref d, ref ch) in arb_digle_and_change(20, 10)) {
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

// Creates an arbitrary digle and a sequence of changes, which can be applied to the digle
// one-by-one.
fn arb_digle_and_change_seq(
    initial_size: usize,
    change_size: usize,
    num_changes: usize,
) -> BoxedStrategy<(DigleData, Vec<Changes>)> {
    fn recurse(
        orig: DigleData,
        change_size: usize,
        num_changes: usize,
        cur: DigleData,
        changes: Vec<Changes>,
    ) -> BoxedStrategy<(DigleData, Vec<Changes>)> {
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
    let digle = arb_live_digle(initial_size);
    let num_changes = 1..(num_changes + 1);
    (digle, num_changes)
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
    fn digle_and_change_seq((ref d, ref chs) in arb_digle_and_change_seq(10, 5, 3)) {
        // Apply all the changes one-by-one. At each step, check that reversing the change
        // produces the previous digle.
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
