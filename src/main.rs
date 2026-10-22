//! `lavrac`: compila e roda programas Lavra.

use lavra::diagnostics::{Category, Diagnostic, Style, render};
use lavra::span::SourceFile;
use std::io::IsTerminal;
use std::process::ExitCode;

const USAGE: &str = "\
uso: lavrac <comando> <arquivo.lav>

comandos:
  run     interpreta o programa
  check   só confere gramática, tipos e posse
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, path) = match args.as_slice() {
        [cmd, path] => (cmd.as_str(), path.as_str()),
        [path] if path.ends_with(".lav") => ("run", path.as_str()),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("lavrac: não consegui ler {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let src = SourceFile::new(path, text);
    let style = Style { color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() };
    let report = |d: &Diagnostic| eprint!("{}", render(d, &src, style));
    let (prog, warnings) = match lavra::compile(&src.text) {
        Ok(r) => r,
        Err(d) => {
            report(&d);
            return ExitCode::from(1);
        }
    };
    for w in &warnings {
        report(w);
    }
    match cmd {
        "check" => ExitCode::SUCCESS,
        "run" => {
            // Pilha grande: o interpretador é recursivo e a recursão da Lavra vai fundo.
            let result = std::thread::scope(|s| {
                std::thread::Builder::new()
                    .stack_size(1 << 30)
                    .spawn_scoped(s, || {
                        let stdout = std::io::stdout().lock();
                        let stdin = std::io::stdin().lock();
                        let mut m = lavra::interp::Machine::new(&prog, std::io::BufWriter::new(stdout), stdin);
                        m.echo = std::env::var_os("LAVRA_ECO").is_some();
                        m.run()
                    })
                    .expect("thread do interpretador")
                    .join()
                    .expect("interpretador")
            });
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(f) if f.desist => {
                    report(&Diagnostic::new(Category::Failure, f.message, f.span).primary("o programa desistiu aqui"));
                    ExitCode::from(1)
                }
                Err(f) => {
                    report(&Diagnostic::new(Category::Runtime, f.message, f.span));
                    ExitCode::from(101)
                }
            }
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
