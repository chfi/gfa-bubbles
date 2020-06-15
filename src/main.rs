use gfa::gfa::GFA;
use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::process::exit;

use handlegraph::handlegraph::{edges_iter, handle_edges_iter, handle_iter, HandleGraph};
use handlegraph::hashgraph::*;
use handlegraph::pathgraph::{occurrences_iter, paths_iter, PathHandleGraph};

fn usage(name: &str) {
    println!("Usage: {} <path-to-gfa>", name);
    exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
    }

    let path = PathBuf::from(args[1].clone());
    if let Some(gfa) = parse_gfa(&path) {
        let graph = HashGraph::from_gfa(&gfa);
    } else {
        usage(&args[0]);
    }
}
