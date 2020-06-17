use gfa::gfa::GFA;
use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::collections::BTreeSet;
use std::collections::VecDeque;

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

fn bubble_starts<T: HandleGraph>(graph: &T) -> Vec<NodeId> {
    let mut starts = Vec::new();

    let mut visited: BTreeSet<NodeId> = BTreeSet::new();

    let mut deque: VecDeque<NodeId> = VecDeque::new();

    deque.push_back(NodeId::from(1));

    while let Some(nid) = deque.pop_front() {
        let h = Handle::pack(nid, false);
        if !visited.contains(&nid) {
            if graph.get_degree(&h, Direction::Right) > 1 {
                println!("start at {:?}", nid);
                starts.push(nid)
            }
            handle_edges_iter(graph, h, Direction::Right).for_each(|h| {
                deque.push_back(h.id());
                visited.insert(nid);
            });
        }
        visited.insert(nid);
    }

    starts
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
    }

    let path = PathBuf::from(args[1].clone());
    if let Some(gfa) = parse_gfa(&path) {
        let graph = HashGraph::from_gfa(&gfa);

        let starts = bubble_starts(&graph);

        println!("Found {} bubble starts", starts.len());

    // let pet = handlegraph_to_pet(&graph);

    // let mut dfs = Dfs::new(&pet, NodeIndex::new(1));
    // while let Some(nx) = dfs.next(&pet) {
    //     println!("{:?}", nx);
    // }

    // let mst = min_spanning_tree(&pet);

    // for n in &mst {
    //     println!("{:?}", n);
    // }
    // let mst_graph: DiGraph<u32, ()> = Graph::from_elements(mst);

    // let mut dfs = Dfs::new(&mst_graph, NodeIndex::new(1));
    // while let Some(nx) = dfs.next(&mst_graph) {
    //     println!("{:?}", nx);
    // }

    // let mut bfs = Bfs::new(&mst_graph, NodeIndex::new(1));
    // while let Some(nx) = bfs.next(&mst_graph) {
    //     println!("{:?}", nx);
    // }
    } else {
        usage(&args[0]);
    }
}

fn usage(name: &str) {
    println!("Usage: {} <path-to-gfa>", name);
    exit(1);
}
