use graph::Graph;
use itertools::Itertools;
use multimap::MMap;
use std::collections::{BTreeMap, HashSet};

use crate::NodeId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Digle {
    // TODO: allow retrieving liveness of NodeIds and type of edges.
    chains: Vec<Vec<NodeId>>,
    edges: MMap<usize, usize>,
    clusters: Vec<HashSet<usize>>,
}

// Assumes that `node` is not part of a cycle. Therefore, it is on a chain if and only if it
// has at most one in-neighbor and at most one out-neighbor.
fn on_chain<G: Graph>(g: &G, node: &G::Node) -> bool {
    g.out_edges(node).take(2).count() <= 1 && g.in_edges(node).take(2).count() <= 1
}

// Follows a chain backwards to find the first node on it.
fn chain_first<G: Graph>(g: &G, node: &G::Node) -> G::Node {
    if !on_chain(g, node) {
        *node
    } else {
        let mut ret = *node;
        while let Some(prev) = g.in_neighbors(&ret).next() {
            if on_chain(g, &prev) {
                ret = prev;
            } else {
                return ret;
            }
        }
        ret
    }
}

fn collect_chain<G: Graph>(g: &G, first: &G::Node) -> Vec<G::Node> {
    let mut ret = Vec::new();
    let mut cur = *first;
    ret.push(cur);
    if !on_chain(g, &cur) {
        return ret;
    }

    while let Some(next) = g.out_neighbors(&cur).next() {
        if on_chain(g, &next) {
            ret.push(next);
            cur = next;
        } else {
            break;
        }
    }
    ret
}

impl Digle {
    pub fn num_chains(&self) -> usize {
        self.chains.len()
    }

    pub fn chain(&self, i: usize) -> &[NodeId] {
        &self.chains[i]
    }

    pub fn clusters(&self) -> impl Iterator<Item = &HashSet<usize>> {
        self.clusters.iter()
    }

    pub fn from_graph<G: Graph<Node = NodeId>>(g: G) -> Digle
    where
        G::Edge: graph::Edge<NodeId>,
    {
        let sccs = g.tarjan();

        // The collection of all nodes whose SCC has only one component. These are the ones that
        // can potentially belong to chains.
        let mut singles = sccs
            .parts()
            .filter(|part| part.len() == 1)
            .flat_map(|part| part.iter())
            .collect::<HashSet<_>>();

        // An iterator over nodes in large SCCs.
        let (others1, others2) = sccs
            .parts()
            .filter(|part| part.len() > 1)
            .flat_map(|part| part.iter())
            .cloned()
            .tee();

        // All nodes in larger SCCs get added to the final graph as length-1 chains.
        let mut chains = others1.map(|node| vec![node]).collect::<Vec<_>>();

        // Map going from NodeId to index in the Vec<Node>.
        let mut node_part = others2
            .enumerate()
            .map(|(x, y)| (y, x))
            .collect::<BTreeMap<NodeId, usize>>();

        while !singles.is_empty() {
            let u = chain_first(&g, singles.iter().next().unwrap());
            let chain = collect_chain(&g, &u);
            for v in &chain {
                singles.remove(v);
                node_part.insert(*v, chains.len());
            }
            chains.push(chain);
        }

        let mut edges = MMap::new();

        for u in g.nodes() {
            for v in g.out_neighbors(&u) {
                let u_idx = node_part[&u];
                let v_idx = node_part[&v];

                // Nodes with the same index belong to the same chain, so we don't need to store an
                // edge between them.
                if u_idx != v_idx {
                    edges.insert(u_idx, v_idx);
                }
            }
        }

        let clusters = sccs
            .parts()
            .filter(|part| part.len() > 1)
            .map(|part| part.iter().map(|id| node_part[id]).collect::<HashSet<_>>())
            .collect::<Vec<_>>();

        Digle {
            chains,
            edges,
            clusters,
        }
    }
}

impl Graph for Digle {
    type Node = usize;
    type Edge = usize;

    fn nodes(&'_ self) -> Box<dyn Iterator<Item = usize> + '_> {
        Box::new(0..self.chains.len())
    }

    fn out_edges(&'_ self, u: &usize) -> Box<dyn Iterator<Item = usize> + '_> {
        Box::new(self.edges.get(u).cloned())
    }

    // TODO: consider removing in_edges from the Graph trait and making it part of a different
    // trait.
    fn in_edges(&'_ self, _u: &usize) -> Box<dyn Iterator<Item = usize> + '_> {
        panic!("in-edges not implemented for this graph");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::storage::digle::tests::{arb_live_digle, make_digle};

    #[test]
    fn diamond() {
        let digle = make_digle("0-1, 0-2, 1-3, 2-3");
        let decomp = super::Digle::from_graph(digle.as_digle().as_live_graph());
        assert_eq!(decomp.chains.len(), 4);
        for ch in &decomp.chains {
            assert_eq!(ch.len(), 1);
        }
    }

    proptest! {
        // Checks that the chains of the decomposition form a partition of the original node set.
        #[test]
        fn partition(ref d in arb_live_digle(20)) {
            let decomp = super::Digle::from_graph(d.as_digle().as_live_graph());
            let decomp_nodes = decomp.chains.iter().flat_map(|chain| chain.iter()).cloned().collect::<Vec<_>>();
            let decomp_node_set = decomp_nodes.iter().cloned().collect::<HashSet<_>>();

            // Check that there are no repeated nodes.
            assert_eq!(decomp_nodes.len(), decomp_node_set.len());

            // Check that we got all the nodes once.
            assert_eq!(decomp_nodes.len(), d.as_digle().nodes().count());
        }
    }
}
