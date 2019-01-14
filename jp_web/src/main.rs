extern crate jp_web;
extern crate yew;

use jp_web::textarea::Model;
use yew::prelude::*;

fn main() {
    web_logger::init();
    yew::initialize();
    App::<Model>::new().mount_to_body();
    yew::run_loop();
}
