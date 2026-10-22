//! Harness da 3ª pessoa: lê infinitivos do stdin (um por linha) e imprime
//! `infinitivo<TAB>singular<TAB>plural` (vazio quando a função devolve `None`).

#[allow(dead_code)]
#[path = "../../../src/morphology/verbs/third_person.rs"]
mod m;

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let inf = line.unwrap();
        let inf = inf.trim();
        if inf.is_empty() {
            continue;
        }
        match m::third_person(inf) {
            Some((s, p)) => writeln!(out, "{inf}\t{s}\t{p}").unwrap(),
            None => writeln!(out, "{inf}\t\t").unwrap(),
        }
    }
}
