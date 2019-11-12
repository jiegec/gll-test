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
    Ret,
    L0,
    LS,
    LS_0,   // S -> . A S d
    LS_0_1, // S -> A . S d
    LS_0_2, // S -> A S . d
    L3,     // S -> A S d .
    LS_1,   // S -> . B S
    LS_1_1, // S -> B . S
    LS_1_2, // S -> B S .
    LS_2,   // S -> .
    LA,
    LA_3, // A -> . a
    L6,   // A -> a .
    LA_4, // A -> . c
    L7,   // A -> c .
    LB,
    LB_5,   // B -> . a
    LB_5_1, // B -> a .
    LB_6,   // B -> . b
    L9,     // B -> b .
}

type GSSNode<L> = (L, usize);

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

trait GrammarSymbol {
    fn is_eps(&self) -> bool;
}

impl GrammarSymbol for Symbol {
    fn is_eps(&self) -> bool {
        *self == Symbol::Eps
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SPPFNode<L, S> {
    Dummy,
    // usize, usize: from, to
    // Vec<SPPFNodeIndex>: children
    Symbol(S, usize, usize, Vec<SPPFNodeIndex>),
    Intermediate(L, usize, usize, Vec<SPPFNodeIndex>),
    Packed(L, usize, Vec<SPPFNodeIndex>),
}

trait GrammarLabel {
    type Symbol: PartialEq + GrammarSymbol;
    fn first(&self) -> bool;
    // return Some(lhs) if it is the end
    fn end(&self) -> Option<Self::Symbol>;
}

impl GrammarLabel for Label {
    type Symbol = Symbol;
    fn first(&self) -> bool {
        use Label::*;
        [LS_0_1, LS_1_1].contains(self)
    }

    fn end(&self) -> Option<Symbol> {
        use Label::*;
        use Symbol::*;
        match self {
            L3 => Some(NS),
            LS_1_2 => Some(NS),
            LS_2 => Some(NS),
            L6 => Some(NA),
            L7 => Some(NA),
            LB_5_1 => Some(NB),
            L9 => Some(NB),
            _ => None,
        }
    }
}

impl<L, S> SPPFNode<L, S> {
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
    sppf_nodes: Vec<SPPFNode<L, L::Symbol>>,
    initial_node_index: NodeIndex,
    visited: Vec<BTreeSet<(L, NodeIndex, SPPFNodeIndex)>>, // U_j
    todo: Vec<(L, NodeIndex, usize, SPPFNodeIndex)>,       // R
    pop: BTreeSet<(NodeIndex, SPPFNodeIndex)>,             // P
    current_position: usize,                               // C_i
    current_node_index: NodeIndex,                         // C_u
    current_sppf_node: usize,                              // C_n
}

impl<L: Ord + Clone + GrammarLabel> GSSState<L> {
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
            let edge_data: Vec<(NodeIndex, SPPFNodeIndex)> = edges
                .iter()
                .map(|edge| (edge.target(), *edge.weight()))
                .collect();
            for (v, w) in edge_data {
                let y = self.get_node_p(l.clone(), w, z);
                self.add(l.clone(), v, i, y);
            }
        }
    }

    fn create(&mut self, l: L, u: NodeIndex, j: usize, w: SPPFNodeIndex) -> NodeIndex {
        let node = (l.clone(), j);
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
                    let y = self.get_node_p(l.clone(), w, z);
                    let h = self.sppf_nodes[z].right_extent();
                    self.add(l.clone(), u, h, y);
                }
            }
        }
        v
    }

    fn get_node_t(&mut self, x: L::Symbol, i: usize) -> SPPFNodeIndex {
        let h = if x.is_eps() { i } else { i + 1 };
        self.find_or_create_sppf_symbol(x, i, h)
    }

    fn get_node_p(&mut self, l: L, w: SPPFNodeIndex, z: SPPFNodeIndex) -> SPPFNodeIndex {
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

    fn find_or_create_sppf_symbol(&mut self, s: L::Symbol, i: usize, j: usize) -> SPPFNodeIndex {
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
        loop {
            println!("{:?} {:?}", current_label, state.todo);
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
                            LS_0,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'a', b'b'].contains(&input[state.current_position]) {
                        state.add(
                            LS_1,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if true {
                        state.add(
                            LS_2,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0;
                }
                LS_0 => {
                    state.current_node_index = state.create(
                        LS_0_1,
                        state.current_node_index,
                        state.current_position,
                        state.current_sppf_node,
                    );
                    current_label = LA;
                }
                LS_0_1 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[state.current_position]) {
                        state.current_node_index = state.create(
                            LS_0_2,
                            state.current_node_index,
                            state.current_position,
                            state.current_sppf_node,
                        );
                        current_label = LS;
                    } else {
                        current_label = L0;
                    }
                }
                LS_0_2 => {
                    if input[state.current_position] == b'd' {
                        let right = state.get_node_t(Symbol::TD, state.current_position);
                        state.current_position += 1;
                        state.current_sppf_node =
                            state.get_node_p(L3, state.current_sppf_node, right);
                        current_label = Ret;
                    } else {
                        current_label = L0;
                    }
                }
                LS_1 => {
                    state.current_node_index = state.create(
                        LS_1_1,
                        state.current_node_index,
                        state.current_position,
                        state.current_sppf_node,
                    );
                    current_label = LB;
                }
                LS_1_1 => {
                    if [b'a', b'b', b'c', b'd', b'$'].contains(&input[state.current_position]) {
                        state.current_node_index = state.create(
                            LS_1_2,
                            state.current_node_index,
                            state.current_position,
                            state.current_sppf_node,
                        );
                        current_label = LS;
                    } else {
                        current_label = L0;
                    }
                }
                LS_1_2 => {
                    current_label = Ret;
                }
                LS_2 => {
                    let right = state.get_node_t(Symbol::Eps, state.current_position);
                    state.current_sppf_node =
                        state.get_node_p(Label::LS_2, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LA => {
                    if [b'a'].contains(&input[state.current_position]) {
                        state.add(
                            LA_3,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'c'].contains(&input[state.current_position]) {
                        state.add(
                            LA_4,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0
                }
                LA_3 => {
                    let right = state.get_node_t(Symbol::TA, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.get_node_p(L6, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LA_4 => {
                    let right = state.get_node_t(Symbol::TC, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.get_node_p(L7, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LB => {
                    if [b'a'].contains(&input[state.current_position]) {
                        state.add(
                            LB_5,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    if [b'b'].contains(&input[state.current_position]) {
                        state.add(
                            LB_6,
                            state.current_node_index,
                            state.current_position,
                            0, // dummy
                        );
                    }
                    current_label = L0
                }
                LB_5 => {
                    let right = state.get_node_t(Symbol::TA, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node =
                        state.get_node_p(LB_5_1, state.current_sppf_node, right);
                    current_label = Ret;
                }
                LB_6 => {
                    let right = state.get_node_t(Symbol::TB, state.current_position);
                    state.current_position += 1;
                    state.current_sppf_node = state.get_node_p(L9, state.current_sppf_node, right);
                    current_label = Ret;
                }
                Ret => {
                    state.pop(
                        state.current_node_index,
                        state.current_position,
                        state.current_sppf_node,
                    );
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
