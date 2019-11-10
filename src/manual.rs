use std::collections::LinkedList;
use std::str;

// SS -> S $
// S -> A S d | B S | eps
// A -> a | c
// B -> a | b
// First(SS) = {a, b, c, eps}
// First(S) = {a, b, c, eps}
// Follow(S) = {d, $}
// First(A) = {a, c}
// First(B) = {a, b}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
enum Label {
    Succ,
    SS, SS_S,
    S,
    S1, S1_A, S1_AS,
    S2, S2_B, S2_BS,
    S3,
    A,
    B,
}

#[derive(Clone, Debug)]
struct Cont {
    label: Label,
    pos: usize,
    stack: Vec<Label>,
}

pub fn parse(input: &[u8]) {
    use Label::*;
    let mut todo: LinkedList<Cont> = LinkedList::new();
    let mut stack: Vec<Label> = Vec::new();
    stack.push(Succ);
    todo.push_back(Cont {
        label: SS,
        pos: 0,
        stack
    });
    loop {
        if let Some(cont) = todo.pop_back() {
            println!("at {} CONT: {:?} STACK: {:?}", cont.pos, cont, cont.stack);
            match cont.label {
                Succ => {
                    println!("Found match! {}", str::from_utf8(&input[..cont.pos]).unwrap());
                }
                SS => {
                    // SS -> .S $
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    stack.push(SS_S);
                    todo.push_back(Cont {
                        label: S,
                        pos,
                        stack
                    })
                }
                SS_S => {
                    if input[cont.pos] == b'$' {
                        let mut pos = cont.pos;
                        let mut stack = cont.stack.clone();
                        pos += 1;
                        let label = stack.pop().unwrap();
                        todo.push_back(Cont {
                            label,
                            pos,
                            stack,
                        });
                    }
                }
                S => {
                    if input[cont.pos] == b'a' || input[cont.pos] == b'c' {
                        // S -> A S d
                        let pos = cont.pos;
                        let stack = cont.stack.clone();
                        todo.push_back(Cont {
                            label: S1,
                            pos,
                            stack
                        });
                    }
                    if input[cont.pos] == b'a' || input[cont.pos] == b'b' {
                        // S -> B S
                        let pos = cont.pos;
                        let stack = cont.stack.clone();
                        todo.push_back(Cont {
                            label: S2,
                            pos,
                            stack
                        });
                    }
                    {
                        // S -> eps
                        let pos = cont.pos;
                        let stack = cont.stack.clone();
                        todo.push_back(Cont {
                            label: S3,
                            pos,
                            stack
                        });
                    }
                },
                S1 => {
                    // S -> .A S d
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    stack.push(S1_A);
                    todo.push_back(Cont {
                        label: A,
                        pos,
                        stack
                    })
                }
                S1_A => {
                    // S -> A .S d
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    stack.push(S1_AS);
                    todo.push_back(Cont {
                        label: S,
                        pos,
                        stack
                    })
                }
                S1_AS => {
                    // S -> A S .d
                    if input[cont.pos] == b'd' {
                        let mut pos = cont.pos;
                        let mut stack = cont.stack.clone();
                        pos += 1;
                        let label = stack.pop().unwrap();
                        todo.push_back(Cont {
                            label,
                            pos,
                            stack,
                        });
                    }
                }
                S2 => {
                    // S -> .B S
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    stack.push(S2_B);
                    todo.push_back(Cont {
                        label: B,
                        pos,
                        stack: stack.clone()
                    })
                }
                S3 => {
                    // S -> .eps
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    let label = stack.pop().unwrap();
                    todo.push_back(Cont {
                        label,
                        pos,
                        stack,
                    });
                }
                S2_B => {
                    // S -> B .S
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    stack.push(S2_BS);
                    todo.push_back(Cont {
                        label: S,
                        pos,
                        stack
                    })
                }
                S2_BS => {
                    // S -> B S.
                    let pos = cont.pos;
                    let mut stack = cont.stack.clone();
                    let label = stack.pop().unwrap();
                    todo.push_back(Cont {
                        label,
                        pos,
                        stack
                    });
                }
                A => {
                    if input[cont.pos] == b'a' {
                        let mut pos = cont.pos;
                        let mut stack = cont.stack.clone();
                        pos += 1;
                        let label = stack.pop().unwrap();
                        todo.push_back(Cont {
                            label,
                            pos,
                            stack,
                        });
                    }
                }
                B => {
                    if input[cont.pos] == b'a' {
                        let mut pos = cont.pos;
                        let mut stack = cont.stack.clone();
                        let label = stack.pop().unwrap();
                        pos += 1;
                        todo.push_back(Cont {
                            label,
                            pos,
                            stack,
                        });
                    }
                    if input[cont.pos] == b'b' {
                        let mut pos = cont.pos;
                        let mut stack = cont.stack.clone();
                        let label = stack.pop().unwrap();
                        pos += 1;
                        todo.push_back(Cont {
                            label,
                            pos,
                            stack
                        });
                    }
                }
            }
        } else {
            break;
        }
    }
}