use gfa::gfa::GFA;
use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use std::process::exit;

use handlegraph::handle::{Direction, Edge, Handle, NodeId};
use handlegraph::handlegraph::{
    edges_iter, handle_edges_iter, handle_iter, HandleGraph,
};
use handlegraph::hashgraph::*;
use handlegraph::pathgraph::{occurrences_iter, paths_iter, PathHandleGraph};

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
            branch_deque.insert(*n, VecDeque::from(vec![*n]));
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

    fn can_continue(&self) -> bool {
        !self.branch_deque.values().all(|dq| dq.is_empty())
    }

    // Iterate each of the search branches one step, returning the set of
    // possible end nodes, if any were found
    fn propagate<T: HandleGraph>(
        &mut self,
        graph: &T,
    ) -> Option<BTreeSet<NodeId>> {
        // take one step on each of the branches

        let branches: Vec<_> = self.branch_deque.keys().cloned().collect();

        let mut possible_ends: BTreeSet<NodeId> = BTreeSet::new();

        // println!("propagating");

        for b_id in branches.into_iter() {
            let b_deq: &mut VecDeque<NodeId> =
                self.branch_deque.get_mut(&b_id).unwrap();

            if let Some(next_id) = b_deq.pop_front() {
                let b_visits: &mut BTreeSet<NodeId> =
                    self.branch_visits.get_mut(&b_id).unwrap();

                if !b_visits.contains(&next_id) {
                    let handle = Handle::pack(next_id, false);

                    let out_degree =
                        graph.get_degree(&handle, Direction::Right);

                    if out_degree == 1 {
                        let possible_end =
                            handle_edges_iter(graph, handle, Direction::Right)
                                .next()
                                .unwrap();
                        possible_ends.insert(possible_end.id());
                        self.branch_ends
                            .get_mut(&b_id)
                            .unwrap()
                            .insert(possible_end.id());
                    }

                    let neighbors: Vec<_> =
                        handle_edges_iter(graph, handle, Direction::Right)
                            .collect();

                    // println!("neighbors: {}", neighbors.len());

                    for h in neighbors {
                        b_deq.push_back(h.id());
                    }
                    // println!("deque next: {}", b_deq.len());

                    b_visits.insert(next_id);
                }
            }
        }

        if possible_ends.is_empty() {
            None
        } else {
            Some(possible_ends)
        }
    }

    // Check if the given node exists in all branch_ends, return the
    // corresponding bubble
    fn check_finished(&self, node: &NodeId) -> Option<Bubble> {
        let count = self.branch_ends.iter().fold(0, |a, (_branch, ends)| {
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

fn find_bubbles<T: HandleGraph>(graph: &T) -> Vec<Bubble> {
    let mut visited: BTreeSet<NodeId> = BTreeSet::new();

    let mut deque: VecDeque<NodeId> = VecDeque::new();

    let mut bubbles: Vec<Bubble> = Vec::new();

    deque.push_back(NodeId::from(1));

    while let Some(nid) = deque.pop_front() {
        let h = Handle::pack(nid, false);
        if !visited.contains(&nid) {
            let rhs_degree = graph.get_degree(&h, Direction::Right);
            if rhs_degree > 1 {
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
                                println!("end at {:?}", b.end);
                                running = false;
                                bubbles.push(b);
                            }
                        }
                    }
                    if !bubble.can_continue() {
                        println!("aborting");
                        running = false;
                    }
                }
            }
            handle_edges_iter(graph, h, Direction::Right).for_each(|h| {
                deque.push_back(h.id());
            });
        }
        visited.insert(nid);
    }
    bubbles
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
    }

    let path = PathBuf::from(args[1].clone());
    if let Some(gfa) = parse_gfa(&path) {
        let graph = HashGraph::from_gfa(&gfa);

        let bubbles = find_bubbles(&graph);

        println!("Found {} bubbles", bubbles.len());

        for b in bubbles {
            println!("{:?} -> {:?}", b.start, b.end);
        }
    } else {
        usage(&args[0]);
    }
}

fn usage(name: &str) {
    println!("Usage: {} <path-to-gfa>", name);
    exit(1);
}
