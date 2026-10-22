//! Harness da verificação do plural: lê uma palavra por linha do stdin e imprime
//! `palavra<TAB>plural`. Compilado por `tools/morphology/check_plural.py`.

#[path = "../../../src/morphology/nouns/plural.rs"]
#[allow(dead_code)]
mod m;

use std::io::{BufRead, BufWriter, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let word = line.expect("stdin");
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        writeln!(out, "{word}\t{}", m::pluralize(word)).expect("stdout");
    }
}
