use crate::agent::{self, JpAgent};

use yew::prelude::*;

pub struct Model {
    value: Option<String>,
    jp: Box<dyn Bridge<JpAgent>>,
}

pub enum Msg {
    Commit,
    None,
    Update(String),
    Disable,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, mut link: ComponentLink<Self>) -> Self {
        info!("Create");
        use crate::agent::Response;
        let callback = link.send_back(|resp| match resp {
            Response::Digle(s) => {
                if let Some(text) = s.file() {
                    Msg::Update(text)
                } else {
                    info!("Not a file: {:?}", s);
                    Msg::Disable
                }
            }
            _ => {
                info!("Unexpected response from jp agent");
                Msg::None
            }
        });

        let mut jp = JpAgent::bridge(callback);
        jp.send(agent::Request::Subscribe(agent::SubscriptionType::Digle));

        Model {
            value: Some("".to_owned()),
            jp: jp,
        }
    }

    fn update(&mut self, msg: Msg) -> ShouldRender {
        match msg {
            Msg::Commit => {
                info!("Commit");
                if let Some(ref text) = self.value {
                    info!("Sending to agent");
                    self.jp.send(agent::Request::UpdateInput(text.clone()));
                } else {
                    // TODO: Figure out how to disable the button when we can't commit.
                    error!("Cannot commit when we're disabled");
                }
            }
            Msg::Update(s) => {
                info!("Update");
                self.value = Some(s);
            }
            Msg::Disable => {
                info!("Disable");
                self.value = None;
            }
            Msg::None => {
                debug!("Got a None message");
            }
        }
        true
    }
}

impl Renderable<Model> for Model {
    fn view(&self) -> Html<Self> {
        html! {
            <div id="editor", >
                <textarea
                    rows=24,
                    value=self.value.as_ref().map(|s| &s[..]).unwrap_or(""),
                    // Probably inefficient like this, need to figure out how to get the value of
                    // the textarea on commit.
                    oninput=|e| Msg::Update(e.value),
                    >
                </textarea>
                <button id="commit", onclick=|_| Msg::Commit,>{ "Commit" }</button>
            </div>
        }
    }
}
