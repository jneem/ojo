#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

use wasm_bindgen::prelude::*;

use libquilt::{EdgeKind, NodeId, PatchId};
use quilt_graph::Graph;
use std::collections::{HashMap, HashSet};

#[wasm_bindgen]
pub struct Repo {
    inner: libquilt::Repo,
}

#[wasm_bindgen]
impl Repo {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Repo {
        console_log::init_with_level(log::Level::Debug).unwrap();
        let inner = libquilt::Repo::init_tmp();

        Repo { inner }
    }

    pub fn commit(&mut self, new_input: &str) {
        match self.inner.diff("master", new_input.as_bytes()) {
            Ok(diff) => {
                let changes = libquilt::Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);
                if !changes.changes.is_empty() {
                    let id = self.inner.create_patch("You", "Msg", changes).unwrap();
                    self.inner.apply_patch("master", &id).unwrap();
                }
            }
            Err(_) => {
                panic!("FIXME: what to do here?");
            }
        }
    }

    pub fn apply_patch(&mut self, patch_id: &str) {
        let patch_id = PatchId::from_base64(patch_id).unwrap();
        self.inner.apply_patch("master", &patch_id).unwrap();
    }

    pub fn unapply_patch(&mut self, patch_id: &str) {
        let patch_id = PatchId::from_base64(patch_id).unwrap();
        self.inner.unapply_patch("master", &patch_id).unwrap();
    }

    pub fn apply_changes(&mut self, changes: &Changes) {
        let id = self
            .inner
            .create_patch("You", "Msg", changes.to_quilt_changes())
            .unwrap();
        self.inner.apply_patch("master", &id).unwrap();
    }

    pub fn file(&self) -> Option<String> {
        let data = self.inner.file("master").ok()?;
        String::from_utf8(data.as_bytes().to_owned()).ok()
    }

    pub fn patches(&self) -> Patches {
        let ids = self.inner.all_patches().cloned().collect::<Vec<_>>();
        let applied_ids = self
            .inner
            .patches("master")
            .cloned()
            .collect::<HashSet<_>>();
        let id_idx = ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(x, y)| (y, x))
            .collect::<HashMap<_, _>>();

        let mut deps = Vec::new();
        let mut patches = Vec::new();

        for p in &ids {
            patches.push(Patch {
                id: p.to_base64(),
                applied: applied_ids.contains(&p),
            });
            for q in self.inner.patch_deps(p) {
                deps.push((id_idx[p], id_idx[q]));
            }
        }

        Patches { patches, deps }
    }

    pub fn graggle(&self) -> Graggle {
        let d = self.inner.graggle("master").unwrap();
        let id_idx = d
            .as_full_graph()
            .nodes()
            .enumerate()
            .map(|(idx, id)| (id, idx))
            .collect::<HashMap<_, _>>();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for u in d.as_full_graph().nodes() {
            nodes.push(GraggleNode {
                id: format!("{}/{}", u.patch.to_base64(), u.node),
                live: d.is_live(&u),
                text: String::from_utf8(self.inner.contents(&u).to_owned()).unwrap(),
            });

            for edge in d.all_out_edges(&u) {
                edges.push(GraggleEdge {
                    from: id_idx[&u],
                    to: id_idx[&edge.dest],
                    pseudo: edge.kind == EdgeKind::Pseudo,
                });
            }
        }

        Graggle { nodes, edges }
    }
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct Patch {
    id: String,
    applied: bool,
}

#[wasm_bindgen]
pub struct Patches {
    patches: Vec<Patch>,
    /// If the pair `(x, y)` is present, it means that patch `x` depends on patch `y`.
    deps: Vec<(usize, usize)>,
}

#[wasm_bindgen]
impl Patches {
    // Returns a vec of strings
    pub fn patches(&self) -> JsValue {
        JsValue::from_serde(&self.patches).unwrap()
    }

    // Returns a vec of pairs
    pub fn deps(&self) -> JsValue {
        JsValue::from_serde(&self.deps).unwrap()
    }
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct GraggleNode {
    id: String,
    text: String,
    live: bool,
}

#[wasm_bindgen]
impl GraggleNode {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn is_live(&self) -> bool {
        self.live
    }
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct GraggleEdge {
    pub from: usize,
    pub to: usize,
    pub pseudo: bool,
}

#[wasm_bindgen]
pub struct Graggle {
    nodes: Vec<GraggleNode>,
    edges: Vec<GraggleEdge>,
}

#[wasm_bindgen]
impl Graggle {
    pub fn nodes(&self) -> JsValue {
        JsValue::from_serde(&self.nodes).unwrap()
    }

    pub fn edges(&self) -> JsValue {
        JsValue::from_serde(&self.edges).unwrap()
    }
}

#[wasm_bindgen]
#[derive(Deserialize)]
pub struct Changes {
    deleted_nodes: Vec<String>,
    added_edges: Vec<(String, String)>,
}

#[wasm_bindgen]
impl Changes {
    /// Creates a new changeset from a list of nodes to be deleted and edges to be added.
    ///
    /// `nodes` should be an array of strings (the ids of the nodes to be deleted) and `edges`
    /// should be an array of pairs of strings (the sources and destinations of the edges to be
    /// added).
    #[wasm_bindgen(constructor)]
    pub fn new(nodes: &JsValue, edges: &JsValue) -> Changes {
        debug!("{:?}", nodes);
        debug!("{:?}", edges);
        Changes {
            deleted_nodes: nodes.into_serde().unwrap(),
            added_edges: edges.into_serde().unwrap(),
        }
    }

    // Converts this into an libquilt::Changes.
    fn to_quilt_changes(&self) -> libquilt::Changes {
        fn node_id(s: &str) -> NodeId {
            let i = s.find('/').unwrap();
            NodeId {
                patch: PatchId::from_base64(&s[..i]).unwrap(),
                node: s[(i + 1)..].parse().unwrap(),
            }
        }
        let nodes = self
            .deleted_nodes
            .iter()
            .map(|node| libquilt::Change::DeleteNode { id: node_id(&node) });

        let edges = self
            .added_edges
            .iter()
            .map(|(src, dest)| libquilt::Change::NewEdge {
                src: node_id(&src),
                dest: node_id(&dest),
            });
        libquilt::Changes {
            changes: nodes.chain(edges).collect(),
        }
    }
}
