use gfa::gfa::GFA;
use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::convert::TryFrom;
use std::process::exit;

use handlegraph::handle::{Direction, Edge, Handle, NodeId};
use handlegraph::handlegraph::{edges_iter, handle_edges_iter, handle_iter, HandleGraph};
use handlegraph::hashgraph::*;
use handlegraph::pathgraph::{occurrences_iter, paths_iter, PathHandleGraph};

use petgraph::algo::{dijkstra, min_spanning_tree};
use petgraph::data::FromElements;
use petgraph::graph::*;
use petgraph::prelude::*;
use petgraph::Graph;

// Generate petgraph from hashgraph
fn handlegraph_to_pet<T: HandleGraph>(graph: &T) -> DiGraph<u32, ()> {
    DiGraph::<u32, ()>::from_edges(edges_iter(graph).map(|e| {
        let Edge(l, r) = e;
        let l = u32::try_from(l.unpack_number()).unwrap();
        let r = u32::try_from(r.unpack_number()).unwrap();
        (l, r)
    }))
}

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
        let pet = handlegraph_to_pet(&graph);

        let mut dfs = Dfs::new(&pet, NodeIndex::new(1));
        while let Some(nx) = dfs.next(&pet) {
            println!("{:?}", nx);
        }
    } else {
        usage(&args[0]);
    }
}
