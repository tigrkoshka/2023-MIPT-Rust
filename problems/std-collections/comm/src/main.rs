#![forbid(unsafe_code)]

use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{BufRead, BufReader},
};

fn read_lines(path: &String) -> HashSet<String> {
    let mut lines_set = HashSet::new();

    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        lines_set.insert(line.unwrap());
    }

    lines_set
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    assert_eq!(args.len(), 3);

    let mut first_lines = read_lines(&args[1]);

    let second_file = File::open(&args[2]).unwrap();
    let second_reader = BufReader::new(second_file);
    for line in second_reader.lines() {
        let unwrapped_line = line.unwrap();
        if first_lines.contains(&unwrapped_line) {
            println!("{}", unwrapped_line);
            first_lines.remove(&unwrapped_line);
        }
    }
}
