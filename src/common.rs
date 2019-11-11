#[derive(Debug)]
pub enum A {
    A,
    C,
}
#[derive(Debug)]
pub enum B {
    A,
    B,
}
#[derive(Debug)]
pub enum S {
    ASd(Box<A>, Box<S>),
    BS(Box<B>, Box<S>),
    Eps,
}
#[derive(Debug)]
pub enum SS {
    S(Box<S>),
}