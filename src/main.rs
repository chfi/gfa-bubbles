use gfa::parser::parse_gfa;

use std::env;
use std::path::PathBuf;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use std::process::exit;

use handlegraph::handle::{Direction, Handle, NodeId};
use handlegraph::handlegraph::{handle_edges_iter, HandleGraph};
use handlegraph::hashgraph::*;

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

                    neighbors.iter().for_each(|h| b_deq.push_back(h.id()));

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

fn find_bubbles<T: HandleGraph>(graph: &T, start: NodeId) -> Vec<Bubble> {
    let mut visited: BTreeSet<NodeId> = BTreeSet::new();

    let mut deque: VecDeque<NodeId> = VecDeque::new();

    let mut bubbles: Vec<Bubble> = Vec::new();

    deque.push_back(start);

    let max_restarts = 100;

    let mut loop_ends: Vec<NodeId> = Vec::new();
    let mut loop_end = None;

    let mut restarts = 0;

    while let Some(nid) = deque.pop_front() {
        let h = Handle::pack(nid, false);

        let mut found_bubble = false;

        if !visited.contains(&nid) {
            let rhs_degree = graph.get_degree(&h, Direction::Right);
            // If we're not in a bubble, but the outbound edges
            // branch, we start a bubble
            if rhs_degree > 1 {
                let neighbors = handle_edges_iter(graph, h, Direction::Right)
                    .map(|h| h.id())
                    .collect();

                let mut bubble = BubbleState::new(rhs_degree, nid, &neighbors);

                while !found_bubble {
                    if let Some(possible_ends) = bubble.propagate(graph) {
                        // if any of the possible ends actually end
                        // the bubble, we're done
                        for end in possible_ends {
                            if let Some(b) = bubble.check_finished(&end) {
                                found_bubble = true;
                                bubbles.push(b);
                            }
                        }
                    }
                    // Aborts if none of the bubble branches can
                    // continue; should only happen if a possible
                    // bubble start is found at the end of the graph,
                    // or if the search is stuck in a loop
                    if !bubble.can_continue() {
                        // For now we assume the search gets stuck in a loop
                        if restarts < max_restarts {
                            if let Some(lnid) = loop_end {
                                deque.push_back(lnid + 1);
                                restarts += 1;
                                loop_end = Some(lnid + 1);
                            } else {
                                loop_end = Some(nid);
                            }
                        }
                        // if restarts < max_restarts {
                        //     deque.push_back(nid + 1);
                        //     restarts += 1;
                        // }
                        // found_bubble = true;
                    }
                }
            }
            loop_end = None;
            restarts = 0;
            // Skip forward if a bubble was found
            if found_bubble {
                deque.push_back(bubbles.last().unwrap().end);
            } else {
                handle_edges_iter(graph, h, Direction::Right)
                    .for_each(|h| deque.push_back(h.id()));
            }
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

        let start_node = args
            .get(2)
            .and_then(|i| i.parse::<u64>().ok())
            .map(NodeId::from)
            .or(Some(NodeId::from(1)))
            .unwrap();

        let mut bubbles = Vec::new();
        let mut start = start_node;
        for _i in 0..10 {
            let mut results = find_bubbles(&graph, start);
            bubbles.append(&mut results);
            match bubbles.last() {
                None => {
                    break;
                }
                Some(n) => {
                    start = n.end + 1;
                    println!("restarting at {:?}", start);
                }
            };
            // start = results.last().unwrap().end + 1;
        }

        println!("# found {} bubbles", bubbles.len());
        println!("start,end");

        for b in bubbles {
            println!("{},{}", u64::from(b.start), u64::from(b.end));
        }
    } else {
        usage(&args[0]);
    }
}

fn usage(name: &str) {
    println!("Usage: {} <path-to-gfa> [optional start node]", name);
    exit(1);
}
