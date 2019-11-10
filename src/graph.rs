use std::collections::{BTreeSet, BTreeMap, VecDeque};
use petgraph::{Graph, Directed, graph::NodeIndex};
use petgraph::dot::Dot;
use std::fs::File;
use std::io::Write;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Label {
    Accept,
    Ret,
    L0,
    LS,
    LS1, L1, L2,
    LS2, L3, L4,
    LS3,
    LA,
    LB,
}

type GSSNode = (Label, usize);

#[derive(Debug)]
struct GSSState {
    graph: Graph<GSSNode, (), Directed>,
    nodes: BTreeMap<GSSNode, NodeIndex>,
    visited: Vec<BTreeSet<(Label, NodeIndex)>>,
    todo: VecDeque<(Label, NodeIndex, usize)>,
    pop: BTreeSet<(NodeIndex, usize)>,
    accept_node_index: NodeIndex,
    current_node_index: NodeIndex,
}

fn add(l: Label, u: NodeIndex, j: usize, state: &mut GSSState) {
    if !state.visited[j].contains(&(l, u)) {
        state.visited[j].insert((l, u));
        state.todo.push_back((l, u, j));
    }
}

fn pop(u: NodeIndex, j: usize, state: &mut GSSState) {
    if u != state.accept_node_index {
        state.pop.insert((u, j));
        let node = state.graph[u];
        let neighbors: Vec<NodeIndex> = state.graph.neighbors(u).collect();
        for v in neighbors {
            add(node.0, v, j, state);
        }
    }
}

fn create(l: Label, u: NodeIndex, j: usize, state: &mut GSSState) {
    let node = (l, j);
    let v = if let Some(index) = state.nodes.get(&node) {
        *index
    } else {
        let index = state.graph.add_node(node);
        state.nodes.insert(node, index);
        index
    };
    if state.graph.find_edge(v, u).is_none() {
        state.graph.add_edge(v, u, ());
        let pop = state.pop.clone();
        for (index, k) in pop.iter() {
            if index == &v {
                add(l, u, *k, state);
            }
        }
    }
    state.current_node_index = v;
}

pub fn parse(input: &[u8]) {
    use Label::*;
    let m = input.len() - 1;
    let mut i = 0;

    let mut graph: Graph<GSSNode, (), Directed> = Graph::new();
    let mut nodes = BTreeMap::new();
    let initial_node = (L0, 0);
    let accept_node = (Accept, 0);
    let initial_node_index = graph.add_node(initial_node);
    let accept_node_index = graph.add_node(accept_node);
    nodes.insert(initial_node, initial_node_index);
    nodes.insert(accept_node, accept_node_index);
    graph.add_edge(initial_node_index, accept_node_index, ());

    let mut state = GSSState {
        graph,
        nodes,
        visited: vec![BTreeSet::new(); input.len()],
        todo: VecDeque::new(),
        pop: BTreeSet::new(),
        accept_node_index,
        current_node_index: initial_node_index,
    };
    // FIRST(S$)
    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[0]) {
        let mut current_label = LS;
        loop {
            match current_label {
                L0 => {
                    if let Some((l, u, j)) = state.todo.pop_front() {
                        current_label = l;
                        state.current_node_index = u;
                        i = j;
                    } else {
                        if state.visited[m].contains(&(L0, accept_node_index)) {
                            println!("Succ");
                            break;
                        } else {
                            println!("Fail");
                            break;
                        }
                    }
                }
                LS => {
                    if [b'a', b'c'].contains(&input[i]) {
                        add(LS1, state.current_node_index, i, &mut state);
                    }
                    if [b'a', b'c'].contains(&input[i]) {
                        add(LS2, state.current_node_index, i, &mut state);
                    }
                    if true {
                        add(LS3, state.current_node_index, i, &mut state);
                    }
                    current_label = L0;
                }
                LS1 => {
                    create(L1, state.current_node_index, i, &mut state);
                    current_label = LA;
                }
                L1 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[i]) {
                        create(L2, state.current_node_index, i, &mut state);
                        current_label = LB;
                    } else { current_label = L0;}
                }
                L2 => {
                    if input[i] == b'd' {
                        i += 1;
                        current_label = Ret;
                    } else {
                        current_label = L0;
                    }
                }
                LS2 => {
                    create(L3, state.current_node_index, i, &mut state);
                    current_label = LB;
                }
                L3 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[i]) {
                        create(L4, state.current_node_index, i, &mut state);
                        current_label = LS;
                    } else { current_label = L0;}
                }
                L4 => {
                    current_label = Ret;
                }
                LS3 => {
                    current_label = Ret;
                }
                LA => {
                    if [b'a', b'c'].contains(&input[i]) {
                        i += 1;
                        current_label = Ret;
                    } else { current_label = L0;}
                }
                LB => {
                    if [b'a', b'b'].contains(&input[i]) {
                        i += 1;
                        current_label = Ret;
                    } else { current_label = L0;}
                }
                Ret => {
                    pop(state.current_node_index, i, &mut state);
                    current_label = L0;
                }
                _ => {
                    break;
                }
            }
        }
        let mut f = File::create("gss.dot").unwrap();
        write!(f, "{:?}", Dot::with_config(&state.graph, &[])).unwrap();
    } else {
        println!("Fail");
        return;
    }
}