//! Harness da 1ª pessoa: lê infinitivos do stdin (um por linha) e imprime
//! `infinitivo<TAB>forma`, ou `infinitivo<TAB>-` quando a função devolve `None`.

#[allow(dead_code)]
#[path = "../../../src/morphology/verbs/first_person.rs"]
mod m;

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let inf = line.trim();
        if inf.is_empty() {
            continue;
        }
        match m::first_person(inf) {
            Some(form) => writeln!(out, "{inf}\t{form}").unwrap(),
            None => writeln!(out, "{inf}\t-").unwrap(),
        }
    }
}
