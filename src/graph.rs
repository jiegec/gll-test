use petgraph::dot::Dot;
use petgraph::{
    graph::{EdgeReference, NodeIndex},
    visit::EdgeRef,
    Directed, Graph,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Label {
    Accept,
    Ret,
    L0,
    LS,
    LS1, // S -> . A S d
    L1,  // S -> A . S d
    L2,  // S -> A S . d
    L3,  // S -> A S d .
    LS2, // S -> . B S
    L4,  // S -> B . S
    L5,  // S -> B S .
    LS3, // S -> .
    LA,
    LA1, // A -> . a
    L6,  // A -> a .
    LA2, // A -> . c
    L7,  // A -> c .
    LB,
    LB1, // B -> . a
    L8,  // B -> a .
    LB2, // B -> . b
    L9,  // B -> b .
}

type GSSNode<L> = (L, usize);

#[derive(Debug, Clone)]
enum S {
    ASd(usize, usize),
    BS(usize, usize),
    Eps,
}

#[derive(Debug, Clone)]
enum A {
    A,
    C,
}

#[derive(Debug, Clone)]
enum B {
    A,
    B,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Symbol {
    // terminals
    TA,
    TB,
    TC,
    TD,
    // non terminals
    NS,
    NA,
    NB,
    // eps
    Eps,
}

type SPPFNodeIndex = usize;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SPPFNode<L> {
    Dummy,
    // usize, usize: from, to
    // Vec<SPPFNodeIndex>: children
    Symbol(Symbol, usize, usize, Vec<SPPFNodeIndex>),
    Intermediate(L, usize, usize, Vec<SPPFNodeIndex>),
    Packed(L, usize, Vec<SPPFNodeIndex>),
}

trait GrammarLabel {
    fn first(&self) -> bool;
    // return Some(lhs) if it is the end
    fn end(&self) -> Option<Symbol>;
}

impl GrammarLabel for Label {
    fn first(&self) -> bool {
        use Label::*;
        [L1, L4].contains(self)
    }

    fn end(&self) -> Option<Symbol> {
        use Label::*;
        use Symbol::*;
        match self {
            L3 => Some(NS),
            L5 => Some(NS),
            LS3 => Some(NS),
            L6 => Some(NA),
            L7 => Some(NA),
            L8 => Some(NB),
            L9 => Some(NB),
            _ => None,
        }
    }
}

impl<L> SPPFNode<L> {
    fn right_extent(&self) -> usize {
        use SPPFNode::*;
        match self {
            Symbol(_, _, r, _) => *r,
            Intermediate(_, _, r, _) => *r,
            _ => unimplemented!(),
        }
    }

    fn left_extent(&self) -> usize {
        use SPPFNode::*;
        match self {
            Symbol(_, l, _, _) => *l,
            Intermediate(_, l, _, _) => *l,
            _ => unimplemented!(),
        }
    }

    fn children(&self) -> Option<&Vec<SPPFNodeIndex>> {
        use SPPFNode::*;
        match self {
            Dummy => None,
            Symbol(_, _, _, children) => Some(children),
            Intermediate(_, _, _, children) => Some(children),
            Packed(_, _, children) => Some(children),
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<SPPFNodeIndex>> {
        use SPPFNode::*;
        match self {
            Symbol(_, _, _, children) => Some(children),
            Intermediate(_, _, _, children) => Some(children),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug)]
struct GSSState<L: Ord + Clone + GrammarLabel> {
    graph: Graph<GSSNode<L>, SPPFNodeIndex, Directed>,
    nodes: BTreeMap<GSSNode<L>, NodeIndex>,
    sppf_nodes: Vec<SPPFNode<L>>,
    initial_node_index: NodeIndex,
    visited: Vec<BTreeSet<(L, NodeIndex, SPPFNodeIndex)>>, // U_j
    todo: Vec<(L, NodeIndex, usize, SPPFNodeIndex)>,       // R
    pop: BTreeSet<(NodeIndex, SPPFNodeIndex)>,             // P
    current_position: usize,                               // C_i
    current_node_index: NodeIndex,                         // C_u
    current_sppf_node: usize,                              // C_n
}

impl<L: Ord + Clone + GrammarLabel + Debug> GSSState<L> {
    fn add(&mut self, l: L, u: NodeIndex, i: usize, w: SPPFNodeIndex) {
        if !self.visited[i].contains(&(l.clone(), u, w)) {
            self.visited[i].insert((l.clone(), u, w));
            self.todo.push((l, u, i, w));
        }
    }

    fn pop(&mut self, u: NodeIndex, i: usize, z: SPPFNodeIndex) {
        if u != self.initial_node_index {
            let (l, _k) = self.graph[u].clone();
            self.pop.insert((u, z));
            let edges: Vec<EdgeReference<SPPFNodeIndex>> = self.graph.edges(u).collect();
            let edge_data: Vec<(NodeIndex, SPPFNodeIndex)> = edges.iter().map(|edge| (edge.target(), *edge.weight())).collect();
            for (v, w) in edge_data {
                let y = self.getNodeP(l.clone(), w, z);
                self.add(l.clone(), v, i, y);
            }
        }
    }

    fn create(&mut self, l: L, u: NodeIndex, j: usize, w: SPPFNodeIndex) -> NodeIndex {
        let node = (l.clone(), j);
        println!("create {:?}", node);
        let v = if let Some(index) = self.nodes.get(&node) {
            *index
        } else {
            let index = self.graph.add_node(node.clone());
            self.nodes.insert(node, index);
            index
        };
        if self.graph.find_edge(v, u).is_none() {
            self.graph.add_edge(v, u, w);
            let pop = self.pop.clone();
            for (index, z) in pop.into_iter() {
                if index == v {
                    let y = self.getNodeP(l.clone(), w, z);
                    let h = self.sppf_nodes[z].right_extent();
                    self.add(l.clone(), u, h, y);
                }
            }
        }
        v
    }

    fn getNodeT(&mut self, x: Symbol, i: usize) -> SPPFNodeIndex {
        let h = if let Symbol::Eps = x { i } else { i + 1 };
        self.find_or_create_sppf_symbol(x, i, h)
    }

    fn getNodeP(&mut self, l: L, w: SPPFNodeIndex, z: SPPFNodeIndex) -> SPPFNodeIndex {
        if l.first() {
            return z;
        } else {
            let node_z = &self.sppf_nodes[z];
            let k = node_z.left_extent();
            let i = node_z.right_extent();
            let node_w = &self.sppf_nodes[w];
            if SPPFNode::Dummy != *node_w {
                // w != $
                let j = node_w.left_extent();
                assert_eq!(node_w.right_extent(), k);
                if let Some(t) = l.end() {
                    // t = X
                    let y = self.find_or_create_sppf_symbol(t, j, i);
                    if let Some(children) = self.sppf_nodes[y].children() {
                        if !children.iter().any(|index| match &self.sppf_nodes[*index] {
                            SPPFNode::Packed(node_l, node_k, _) => *node_l == l && *node_k == k,
                            _ => false,
                        }) {
                            let len = self.sppf_nodes.len();
                            self.sppf_nodes[y].children_mut().unwrap().push(len);
                            self.sppf_nodes.push(SPPFNode::Packed(l, k, vec![w, z]));
                        }
                    } else {
                        unimplemented!()
                    }
                    y
                } else {
                    // t = l
                    let y = self.find_or_create_sppf_intermediate(l.clone(), j, i);
                    if let Some(children) = self.sppf_nodes[y].children() {
                        if !children.iter().any(|index| match &self.sppf_nodes[*index] {
                            SPPFNode::Packed(node_l, node_k, _) => *node_l == l && *node_k == k,
                            _ => false,
                        }) {
                            let len = self.sppf_nodes.len();
                            self.sppf_nodes[y].children_mut().unwrap().push(len);
                            self.sppf_nodes.push(SPPFNode::Packed(l, k, vec![w, z]));
                        }
                    } else {
                        unimplemented!()
                    }
                    y
                }
            } else {
                // w = $
                if let Some(t) = l.end() {
                    // t = X
                    let y = self.find_or_create_sppf_symbol(t, k, i);
                    if let Some(children) = self.sppf_nodes[y].children() {
                        if !children.iter().any(|index| match &self.sppf_nodes[*index] {
                            SPPFNode::Packed(node_l, node_k, _) => *node_l == l && *node_k == k,
                            _ => false,
                        }) {
                            let len = self.sppf_nodes.len();
                            self.sppf_nodes[y].children_mut().unwrap().push(len);
                            self.sppf_nodes.push(SPPFNode::Packed(l, k, vec![z]));
                        }
                    } else {
                        unimplemented!()
                    }
                    y
                } else {
                    // t = l
                    let y = self.find_or_create_sppf_intermediate(l.clone(), k, i);
                    if let Some(children) = self.sppf_nodes[y].children() {
                        if !children.iter().any(|index| match &self.sppf_nodes[*index] {
                            SPPFNode::Packed(node_l, node_k, _) => *node_l == l && *node_k == k,
                            _ => false,
                        }) {
                            let len = self.sppf_nodes.len();
                            self.sppf_nodes[y].children_mut().unwrap().push(len);
                            self.sppf_nodes.push(SPPFNode::Packed(l, k, vec![z]));
                        }
                    } else {
                        unimplemented!()
                    }
                    y
                }
            }
        }
    }

    fn find_or_create_sppf_symbol(&mut self, s: Symbol, i: usize, j: usize) -> SPPFNodeIndex {
        for (index, node) in self.sppf_nodes.iter().enumerate() {
            if let SPPFNode::Symbol(node_s, node_i, node_j, _) = node {
                if *node_s == s && *node_i == i && *node_j == j {
                    return index;
                }
            }
        }
        self.sppf_nodes.push(SPPFNode::Symbol(s, i, j, vec![]));
        self.sppf_nodes.len() - 1
    }

    fn find_or_create_sppf_intermediate(&mut self, l: L, i: usize, j: usize) -> SPPFNodeIndex {
        for (index, node) in self.sppf_nodes.iter().enumerate() {
            if let SPPFNode::Intermediate(node_l, node_i, node_j, _) = node {
                if *node_l == l && *node_i == i && *node_j == j {
                    return index;
                }
            }
        }
        self.sppf_nodes
            .push(SPPFNode::Intermediate(l, i, j, vec![]));
        self.sppf_nodes.len() - 1
    }
}

pub fn parse(input: &[u8]) {
    use Label::*;
    let m = input.len() - 1;

    let mut graph: Graph<GSSNode<Label>, SPPFNodeIndex, Directed> = Graph::new();
    let mut nodes = BTreeMap::new();
    let initial_node = (L0, 0);
    let initial_node_index = graph.add_node(initial_node);
    nodes.insert(initial_node, initial_node_index);

    let mut state = GSSState {
        graph,
        nodes,
        sppf_nodes: vec![SPPFNode::Dummy],
        initial_node_index,
        visited: vec![BTreeSet::new(); input.len()],
        todo: Vec::new(),
        pop: BTreeSet::new(),
        current_node_index: initial_node_index,
        current_sppf_node: 0,
        current_position: 0,
    };

    // FIRST(S$)
    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[0]) {
        let mut current_label = LS;
        let mut last_label = Ret;
        loop {
            if current_label != Ret {
                last_label = current_label;
            }
            match current_label {
                L0 => {
                    if let Some((l, u, i, w)) = state.todo.pop() {
                        current_label = l;
                        state.current_node_index = u;
                        state.current_position = i;
                        state.current_sppf_node = w;
                    } else {
                        if state.sppf_nodes.iter().any(|node| {
                            if let SPPFNode::Symbol(Symbol::NS, 0, node_m, _) = node {
                                *node_m == m
                            } else {
                                false
                            }
                        }) {
                            println!("Succ");

                            let mut file = File::create("sppf.dot").unwrap();
                            write!(file, "digraph {{\n").unwrap();
                            for (i, node) in state.sppf_nodes.iter().enumerate() {
                                let label = match node {
                                    SPPFNode::Symbol(s, _, _, _) => format!("{:?}", s),
                                    SPPFNode::Intermediate(_, _, _, _) => format!("I"),
                                    SPPFNode::Packed(_, _, _) => format!("P"),
                                    SPPFNode::Dummy => format!("D"),
                                };
                                write!(file, "{} [label={:?}]\n", i, label).unwrap();
                                if let Some(children) = node.children() {
                                    for child in children {
                                        write!(file, "{} -> {}\n", i, child).unwrap();
                                    }
                                }
                            }
                            write!(file, "}}").unwrap();
                            break;
                        } else {
                            println!("Fail");
                            break;
                        }
                    }
                }
                LS => {
                    if [b'a', b'c'].contains(&input[state.current_position]) {
                        state.add(
                            LS1,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'a', b'b'].contains(&input[state.current_position]) {
                        state.add(
                            LS2,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if true {
                        state.add(
                            LS3,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0;
                }
                LS1 => {
                    state.current_node_index = state.create(
                        L1,
                        state.current_node_index,
                        state.current_position,
                        state.current_sppf_node,
                    );
                    current_label = LA;
                }
                L1 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[state.current_position]) {
                        state.current_node_index = state.create(
                            L2,
                            state.current_node_index,
                            state.current_position,
                            state.current_sppf_node,
                        );
                        current_label = LS;
                    } else {
                        current_label = L0;
                    }
                }
                L2 => {
                    if input[state.current_position] == b'd' {
                        let right = state.getNodeT(Symbol::TD, state.current_position);
                        state.current_position += 1;
                        state.current_sppf_node = state.getNodeP(L3, state.current_sppf_node, right);
                        current_label = Ret;
                    } else {
                        current_label = L0;
                    }
                }
                LS2 => {
                    state.current_node_index = state.create(
                        L4,
                        state.current_node_index,
                        state.current_position,
                        state.current_sppf_node,
                    );
                    current_label = LB;
                }
                L4 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[state.current_position]) {
                        state.current_node_index = state.create(
                            L5,
                            state.current_node_index,
                            state.current_position,
                            state.current_sppf_node,
                        );
                        current_label = LS;
                    } else {
                        current_label = L0;
                    }
                }
                L5 => {
                    current_label = Ret;
                }
                LS3 => {
                    let right = state.getNodeT(Symbol::Eps, state.current_position);
                    state.current_sppf_node = state.getNodeP(Label::LS3, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LA => {
                    if [b'a'].contains(&input[state.current_position]) {
                        state.add(
                            LA1,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'c'].contains(&input[state.current_position]) {
                        state.add(
                            LA2,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0
                }
                LA1 => {
                    let right = state.getNodeT(Symbol::TA, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.getNodeP(L6, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LA2 => {
                    let right = state.getNodeT(Symbol::TC, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.getNodeP(L7, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LB => {
                    if [b'a'].contains(&input[state.current_position]) {
                        state.add(
                            LB1,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'b'].contains(&input[state.current_position]) {
                        state.add(
                            LB2,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0
                }
                LB1 => {
                    let right = state.getNodeT(Symbol::TA, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.getNodeP(L8, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LB2 => {
                    let right = state.getNodeT(Symbol::TB, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.getNodeP(L9, state.current_sppf_node, right);
                    current_label = Ret;
                }
                Ret => {
                    println!(
                        "Ret {:?}",
                        last_label,
                    );
                    state.pop(state.current_node_index, state.current_position, state.current_sppf_node);
                    current_label = L0;
                }
                _ => {
                    panic!("Unreachable {:?}", current_label);
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
