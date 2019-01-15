extern crate jp_web;
extern crate yew;

use jp_web::{graph_view, textarea};
use stdweb::web::{document, IParentNode};
use yew::prelude::*;

fn main() {
    web_logger::init();
    yew::initialize();

    let text_pane = document().query_selector("#text_pane").unwrap().unwrap();
    let graph_pane = document().query_selector("#graph_pane").unwrap().unwrap();
    App::<textarea::Model>::new().mount(text_pane);
    App::<graph_view::Model>::new().mount(graph_pane);
    yew::run_loop();
}
