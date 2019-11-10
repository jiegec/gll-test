use gll_test::graph::parse;
use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        parse(line.unwrap().as_bytes());
    }
}
