use crate::agent::{self, JpAgent, Patches};

use libjp::PatchId;
use std::collections::HashMap;
use yew::prelude::*;

pub struct Model {
    value: Option<Patches>,
    jp: Box<dyn Bridge<JpAgent>>,
}

fn node_id(p: &PatchId) -> String {
    format!("{}", &p.to_base64()[0..8])
}

#[derive(Debug, Serialize)]
struct NodeForJs {
    id: usize,
    label: String,
}

#[derive(Debug, Serialize)]
struct EdgeForJs {
    from: usize,
    to: usize,
}

impl Model {
    fn graph_js(&self) -> Option<(Vec<NodeForJs>, Vec<EdgeForJs>)> {
        let patches = self.value.as_ref()?;
        let mut node_map = HashMap::new();
        let nodes = patches
            .ids
            .iter()
            .enumerate()
            .map(|(i, jp_node_id)| {
                node_map.insert(*jp_node_id, i);
                NodeForJs {
                    id: i,
                    label: node_id(jp_node_id),
                }
            })
            .collect::<Vec<_>>();

        let edges = patches
            .deps
            .iter()
            // If x depends on y, we want an edge going from y to x.
            .map(|&(ref x, ref y)| EdgeForJs {
                from: node_map[y],
                to: node_map[x],
            })
            .collect::<Vec<_>>();

        Some((nodes, edges))
    }
}

pub enum Msg {
    None,
    Update(Patches),
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        info!("create");

        use crate::agent::Response;
        let callback = link.send_back(|resp| match resp {
            Response::Patches(ps) => Msg::Update(ps),
            _ => {
                info!("Unexpected response from jp agent");
                Msg::None
            }
        });

        let mut jp = JpAgent::bridge(callback);
        jp.send(agent::Request::Subscribe(
            agent::SubscriptionType::PatchGraph,
        ));

        Model {
            value: None,
            jp: jp,
        }
    }

    fn update(&mut self, msg: Msg) -> ShouldRender {
        match msg {
            Msg::Update(ps) => {
                self.value = Some(ps);
                true
            }
            Msg::None => {
                debug!("Got a None message");
                false
            }
        }
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        if let Some((nodes, edges)) = self.graph_js() {
            let nodes_str = serde_json::to_string(&nodes).unwrap();
            let edges_str = serde_json::to_string(&edges).unwrap();
            info!("{:?}", nodes);
            info!("{:?}", edges);
            js! {
                render_graph(@{nodes_str}, @{edges_str});
            };
            //var g = new dagreD3.graphlib.json.read(@{graph_json});
            //
            /*
                console.log(g);
                var svg = d3.select("svg");
                var inner = svg.select("g");
                var render = new dagreD3.render();
                console.log("log4 from JS");
                render(inner, g);
                console.log("log5 from JS");
            };
            */
        }
        html! {
            <div> </div>
            //<svg id="graph_view", width=400, height=600,></svg>
        }
    }
}
