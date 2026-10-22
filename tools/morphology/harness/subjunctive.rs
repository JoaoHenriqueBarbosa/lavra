//! Harness de verificação do futuro do subjuntivo: lê infinitivos do stdin, um por
//! linha, e imprime `infinitivo<TAB>forma` para cada um.

#[path = "../../../src/morphology/verbs/subjunctive.rs"]
#[allow(dead_code)]
mod m;

use std::io::{BufRead, BufWriter, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in stdin.lock().lines() {
        let inf = line.expect("stdin legível");
        let inf = inf.trim();
        if inf.is_empty() {
            continue;
        }
        writeln!(out, "{inf}\t{}", m::future_subjunctive(inf)).expect("stdout gravável");
    }
}
