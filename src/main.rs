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
use handlegraph::handlegraph::{
    edges_iter, handle_edges_iter, handle_iter, HandleGraph,
};
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
    #[allow(dead_code)]
    fn visit_from_branch(
        &mut self,
        from: NodeId,
        visited: NodeId,
        neighbors: &Vec<NodeId>,
    ) {
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

    fn can_continue(&self) -> bool {
        self.branch_deque.values().all(|dq| dq.is_empty())
    }

    // Returns the possibly detected ends
    fn propagate<T: HandleGraph>(
        &mut self,
        graph: &T,
    ) -> Option<BTreeSet<NodeId>> {
        // take one step on each of the branches

        // println!("propagating");
        let branches: Vec<_> = self.branch_deque.keys().cloned().collect();

        let mut possible_ends: BTreeSet<NodeId> = BTreeSet::new();

        for b_id in branches.into_iter() {
            let b_deq: &mut VecDeque<NodeId> =
                self.branch_deque.get_mut(&b_id).unwrap();
            println!("deq len: {}", b_deq.len());

            if let Some(next_id) = b_deq.pop_front() {
                println!("branch {:?}\t visiting {:?}", b_id, next_id);
                let b_visits: &mut BTreeSet<NodeId> =
                    self.branch_visits.get_mut(&b_id).unwrap();

                if !b_visits.contains(&next_id) {
                    let handle = Handle::pack(next_id, false);

                    if graph.get_degree(&handle, Direction::Left) > 1 {
                        println!("possible end");
                        possible_ends.insert(next_id);
                        self.branch_ends
                            .get_mut(&b_id)
                            .unwrap()
                            .insert(next_id);
                    }

                    let neighbors =
                        handle_edges_iter(graph, handle, Direction::Right)
                            .filter(|h| !b_visits.contains(&h.id()));

                    for h in neighbors {
                        b_deq.push_back(h.id());
                    }

                    b_visits.insert(next_id);
                }
            }
        }

        if possible_ends.is_empty() {
            None
        } else {
            println!("possible ends: {}", possible_ends.len());
            Some(possible_ends)
        }
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

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct Bubble {
    start: NodeId,
    end: NodeId,
}

fn bubble_starts<T: HandleGraph>(graph: &T) -> Vec<Bubble> {
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

                let mut running = true;

                while running {
                    if let Some(possible_ends) = bubble.propagate(graph) {
                        // if any of the possible ends actually end the
                        // bubble, we're done
                        for end in possible_ends {
                            if let Some(b) = bubble.check_finished(&end) {
                                running = false;
                                bubbles.push(b);
                            }
                        }
                    }
                }

                // TODO loop through all the queues in the
                // bubblestate, traversing the graph and updating the
                // visited sets of each branch

                // whenever a node is found with the same incoming degree as rhs_degree, add it to the branch_ends

                // then check if that node exists across all branch_ends; if it does, the bubble is done

                // current_bubble = Some(BubbleState::new(&neighbors));

                starts.push(nid)
            }
            handle_edges_iter(graph, h, Direction::Right).for_each(|h| {
                deque.push_back(h.id());
            });
        }
        visited.insert(nid);
    }

    bubbles
    // starts
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
