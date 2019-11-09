use futures::executor::block_on;
use futures::future::{BoxFuture, FutureExt};

#[derive(Debug)]
enum A {
    a,
    c,
}
#[derive(Debug)]
enum B {
    a,
    b,
}
#[derive(Debug)]
enum S {
    ASd(Box<A>, Box<S>),
    BS(Box<B>, Box<S>),
    Eps,
}
#[derive(Debug)]
enum SS {
    S(Box<S>),
}

async fn parse_ss(input: &[u8]) -> Option<(SS, usize)> {
    if input[0] == b'a' || input[0] == b'b' || input[0] == b'c' || input[0] == b'$' {
        if let Some((s, len)) = parse_s(&input[..]).await {
            if input[len] == b'$' {
                return Some((SS::S(Box::new(s)), len));
            }
        }
    }
    return None;
}

fn parse_s<'a>(input: &'a [u8]) -> BoxFuture<'a, Option<(S, usize)>> {
    async move {
        if input[0] == b'a' || input[0] == b'c' {
            if let Some((a, len_a)) = parse_a(&input[0..]).await {
                if let Some((s, len_s)) = parse_s(&input[len_a..]).await {
                    if input[len_a + len_s] == b'd' {
                        return Some((S::ASd(Box::new(a), Box::new(s)), 1 + len_a + len_s));
                    }
                }
            }
        }
        if input[0] == b'a' || input[0] == b'b' {
            if let Some((b, len_b)) = parse_b(&input[0..]).await {
                if let Some((s, len_s)) = parse_s(&input[len_b..]).await {
                    return Some((S::BS(Box::new(b), Box::new(s)), len_b + len_s));
                }
            }
        }
        return Some((S::Eps, 0));
    }
    .boxed()
}

async fn parse_a(input: &[u8]) -> Option<(A, usize)> {
    if input[0] == b'a' {
        return Some((A::a, 1));
    }
    if input[0] == b'c' {
        return Some((A::c, 1));
    }
    return None;
}

async fn parse_b(input: &[u8]) -> Option<(B, usize)> {
    if input[0] == b'a' {
        return Some((B::a, 1));
    }
    if input[0] == b'b' {
        return Some((B::b, 1));
    }
    return None;
}

pub fn parse(input: &[u8]) {
    let future = parse_ss(input);
    println!("{:?}", block_on(future));
}
