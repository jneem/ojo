// The js! macro requires lots of recursion.
#![recursion_limit = "1024"]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate yew;

pub mod agent;
pub mod graph_view;
pub mod textarea;
