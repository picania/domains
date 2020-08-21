//#![allow(dead_code)]
//#![allow(unused_variables)]

use std::env;
use std::fs::File;
use std::io::{BufReader, BufRead};

fn main() {
    let args = env::args();
    let count = match args.skip(1).take(1).next() {
        Some(c) => c.parse::<usize>().unwrap(),
        None => 20,
    };
    let database = "zones/ru_domains.ru";

    let file = File::open(database).unwrap();
    let reader = BufReader::new(file);

    reader.lines()
        .filter_map(|x| x.ok())
        .map(|s| {
            s.chars().take_while(|x| !x.is_ascii_whitespace()).collect::<String>()
        })
        .take(count)
        .for_each(|x| {
            println!("{}", x);
        });
}
