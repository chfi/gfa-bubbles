use gfa::gfa::GFA;
use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::collections::BTreeMap;
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

struct BubbleState {
    branch_visits: BTreeMap<NodeId, BTreeSet<NodeId>>,
    branch_deque: BTreeMap<NodeId, VecDeque<NodeId>>,
    branch_ends: BTreeMap<NodeId, BTreeSet<NodeId>>,
    start: NodeId,
    degree: usize,
}

impl BubbleState {
    fn new(degree: usize, start: NodeId, nodes: &Vec<NodeId>) -> BubbleState {
        let mut branch_visits = BTreeMap::new();
        let mut branch_deque = BTreeMap::new();
        let mut branch_ends = BTreeMap::new();
        for n in nodes {
            branch_visits.insert(*n, BTreeSet::new());
            branch_deque.insert(*n, VecDeque::new());
            branch_ends.insert(*n, BTreeSet::new());
        }
        BubbleState {
            degree,
            start,
            branch_visits,
            branch_deque,
            branch_ends,
        }
    }

    // Update the visited set and queue for the branch that started at
    // the node `from`, with the visited node `visited`, which has the
    // right-hand neighbors `neighbors`

    // TODO this should take the LHS-degree of the visited node and update the branch_ends accordingly, if it matches the bubble degree
    fn visit_from_branch(&mut self, from: NodeId, visited: NodeId, neighbors: &Vec<NodeId>) {
        self.branch_visits.entry(from).and_modify(|set| {
            set.insert(visited);
        });

        let visits = self.branch_visits.get(&from).unwrap();
        let to_visit = neighbors.into_iter().filter(|n| !visits.contains(&n));

        self.branch_deque.entry(from).and_modify(|deq| {
            for n in to_visit {
                deq.push_back(*n);
            }
        });
    }

    // if the given node exists in all branch_ends, return the corresponding bubble
    fn check_finished(&self, node: &NodeId) -> Option<Bubble> {
        let count = self.branch_ends.iter().fold(0, |a, (branch, ends)| {
            let ends: &BTreeSet<NodeId> = ends;
            if ends.contains(node) {
                a + 1
            } else {
                a
            }
        });
        if count == self.degree {
            Some(Bubble {
                start: self.start,
                end: *node,
            })
        } else {
            None
        }
    }
}

struct Bubble {
    start: NodeId,
    end: NodeId,
}

fn bubble_starts<T: HandleGraph>(graph: &T) -> Vec<NodeId> {
    let mut starts = Vec::new();

    let mut visited: BTreeSet<NodeId> = BTreeSet::new();

    let mut deque: VecDeque<NodeId> = VecDeque::new();

    let mut current_bubble: Option<BubbleState> = None;
    let mut bubbles: Vec<Bubble> = Vec::new();

    deque.push_back(NodeId::from(1));

    while let Some(nid) = deque.pop_front() {
        let h = Handle::pack(nid, false);
        if !visited.contains(&nid) {
            let rhs_degree = graph.get_degree(&h, Direction::Right);
            if current_bubble.is_none() && rhs_degree > 1 {
                println!("start at {:?}", nid);
                // start a new bubble
                let neighbors = handle_edges_iter(graph, h, Direction::Right)
                    .map(|h| h.id())
                    .collect();

                let mut bubble = BubbleState::new(rhs_degree, nid, &neighbors);

                // TODO loop through all the queues in the
                // bubblestate, traversing the graph and updating the
                // visited sets of each branch

                // whenever a node is found with the same incoming degree as rhs_degree, add it to the branch_ends

                // then check if that node exists across all branch_ends; if it does, the bubble is done

                // current_bubble = Some(BubbleState::new(&neighbors));

                starts.push(nid)
            } else {
                // we're building a bubble,
                let mut state = current_bubble.unwrap();

                current_bubble = Some(state);
                // current_bubble = None;
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
