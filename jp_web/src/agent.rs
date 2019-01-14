use libjp::decomposed_digle::Digle;
use libjp::NodeId;
use std::collections::BTreeMap;
use yew::prelude::worker::*;

#[derive(Debug, Deserialize, Serialize)]
// TODO: better name
pub struct State {
    digle: Digle,
    contents: BTreeMap<NodeId, String>,
}

impl State {
    pub fn file(&self) -> Option<String> {
        if self.digle.num_chains() == 0 {
            Some("".to_owned())
        } else if self.digle.num_chains() == 1 {
            let chain = self.digle.chain(0);
            Some(
                chain
                    .iter()
                    .map(|id| &self.contents[id][..])
                    .collect::<String>(),
            )
        } else {
            None
        }
    }
}

pub struct JpAgent {
    link: AgentLink<JpAgent>,
    repo: libjp::Repo,
    subscriptions: Vec<(HandlerId, SubscriptionType)>,
}

impl JpAgent {
    fn digle_state(&self) -> State {
        let digle = self.repo.digle("master").unwrap();
        let contents = digle
            .nodes()
            .map(|id| {
                (
                    id,
                    String::from_utf8(self.repo.contents(&id).to_owned()).unwrap(),
                )
            })
            .collect::<BTreeMap<NodeId, String>>();
        let digle = Digle::from_digle(digle);
        State { digle, contents }
    }

    fn notify_subscribers(&mut self) {
        info!("notify_subscribers");
        use self::SubscriptionType::*;

        for &(who, typ) in &self.subscriptions {
            match typ {
                Digle => self.link.response(who, Response::Digle(self.digle_state())),
                PatchGraph => self.link.response(who, Response::PatchGraph),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SubscriptionType {
    Digle,
    PatchGraph,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    UpdateInput(String),
    Subscribe(SubscriptionType),
}

impl Transferable for Request {}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Done,
    Error(String),
    Digle(State),
    PatchGraph,
}

impl Transferable for Response {}

pub enum Msg {}

impl Agent for JpAgent {
    // It may make sense to change this to `Public`, but that creates some annoyances with
    // cargo-web, since `Public` implies the need for a separate binary and cargo-web doesn't want
    // to serve more than one.
    type Reach = Context;
    type Message = Msg;
    type Input = Request;
    type Output = Response;

    fn create(link: AgentLink<Self>) -> Self {
        JpAgent {
            link,
            repo: libjp::Repo::init_tmp(),
            subscriptions: Vec::new(),
        }
    }

    // Handle inner messages (of services of `send_back` callbacks)
    fn update(&mut self, _msg: Self::Message) {
        info!("update");
    }

    // Handle incoming messages from components of other agents.
    fn handle(&mut self, msg: Self::Input, who: HandlerId) {
        info!("handle");
        match msg {
            Request::UpdateInput(new_input) => {
                match self.repo.diff("master", new_input.as_bytes()) {
                    Ok(diff) => {
                        let changes =
                            libjp::Changes::from_diff(&diff.file_a, &diff.file_b, &diff.diff);
                        if !changes.changes.is_empty() {
                            let id = self.repo.create_patch("You", "Msg", changes).unwrap();
                            self.repo.apply_patch("master", &id).unwrap();
                        }
                        self.notify_subscribers();
                    }
                    Err(e) => {
                        self.link.response(who, Response::Error(format!("{}", e)));
                    }
                }
            }
            Request::Subscribe(typ) => {
                self.subscriptions.push((who, typ));
            }
        }
    }
}
