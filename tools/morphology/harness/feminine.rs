//! Harness de `adjectives/feminine.rs`: lê linhas `m<TAB>palavra` (pede o feminino)
//! ou `f<TAB>palavra` (pede o masculino) e imprime uma resposta por linha.

#[path = "../../../src/morphology/adjectives/feminine.rs"]
mod m;

use std::io::{BufRead, BufWriter, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        let (dir, word) = line.split_once('\t').expect("linha sem TAB");
        let answer = match dir {
            "m" => m::feminine(word),
            "f" => m::masculine(word),
            _ => panic!("direção desconhecida: {dir}"),
        };
        writeln!(out, "{answer}").expect("stdout");
    }
}
