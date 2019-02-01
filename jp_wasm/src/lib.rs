#[macro_use]
extern crate serde_derive;

use wasm_bindgen::prelude::*;

use graph::Graph;
use libjp::{Changes, EdgeKind, PatchId};
use std::collections::{HashMap, HashSet};

#[wasm_bindgen]
pub struct Repo {
    inner: libjp::Repo,
}

#[wasm_bindgen]
impl Repo {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Repo {
        console_log::init_with_level(log::Level::Debug).unwrap();
        let inner = libjp::Repo::init_tmp();

        Repo { inner }
    }

    pub fn commit(&mut self, new_input: &str) {
        match self.inner.diff("master", new_input.as_bytes()) {
            Ok(diff) => {
                let changes = Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);
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

    pub fn digle(&self) -> Digle {
        let d = self.inner.digle("master").unwrap();
        let id_idx = d
            .as_full_graph()
            .nodes()
            .enumerate()
            .map(|(idx, id)| (id, idx))
            .collect::<HashMap<_, _>>();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for u in d.as_full_graph().nodes() {
            nodes.push(DigleNode {
                id: format!("{}/{}", u.patch.to_base64(), u.node),
                live: d.is_live(&u),
                text: String::from_utf8(self.inner.contents(&u).to_owned()).unwrap(),
            });

            for edge in d.all_out_edges(&u) {
                edges.push(DigleEdge {
                    from: id_idx[&u],
                    to: id_idx[&edge.dest],
                    pseudo: edge.kind == EdgeKind::Pseudo,
                });
            }
        }

        Digle { nodes, edges }
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
pub struct DigleNode {
    id: String,
    text: String,
    live: bool,
}

#[wasm_bindgen]
impl DigleNode {
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
pub struct DigleEdge {
    pub from: usize,
    pub to: usize,
    pub pseudo: bool,
}

#[wasm_bindgen]
pub struct Digle {
    nodes: Vec<DigleNode>,
    edges: Vec<DigleEdge>,
}

#[wasm_bindgen]
impl Digle {
    pub fn nodes(&self) -> JsValue {
        JsValue::from_serde(&self.nodes).unwrap()
    }

    pub fn edges(&self) -> JsValue {
        JsValue::from_serde(&self.edges).unwrap()
    }
}
