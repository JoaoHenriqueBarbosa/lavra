//! `lavrac`: compila e roda programas Lavra.

use lavra::diagnostics::{Diagnostic, Style, render};
use lavra::span::SourceFile;
use std::io::IsTerminal;
use std::process::ExitCode;

const USAGE: &str = "\
uso: lavrac <comando> <arquivo.lav> [-o saída]

comandos:
  run     interpreta o programa
  build   compila para um executável nativo (padrão: o nome do arquivo sem .lav)
  check   só confere gramática, tipos e posse

  lavrac lexicon --json   exporta o léxico da língua (palavras, papéis, documentação)
";

/// O runtime, compilado pelo build.rs e embutido no compilador.
const RUNTIME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/liblavra_rt.a"));

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, flag] = args.as_slice()
        && cmd == "lexicon"
        && flag == "--json"
    {
        print!("{}", lavra::lexicon::to_json());
        return ExitCode::SUCCESS;
    }
    let (cmd, path, output) = match args.as_slice() {
        [cmd, path] => (cmd.as_str(), path.as_str(), None),
        [cmd, path, flag, out] if flag == "-o" => (cmd.as_str(), path.as_str(), Some(out.clone())),
        [path] if path.ends_with(".lav") => ("run", path.as_str(), None),
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
        Err(diags) => {
            for d in &diags {
                report(d);
            }
            return ExitCode::from(1);
        }
    };
    for w in &warnings {
        report(w);
    }
    match cmd {
        "check" => ExitCode::SUCCESS,
        "build" => {
            let out = output.unwrap_or_else(|| path.strip_suffix(".lav").unwrap_or(path).to_string());
            match build(&prog, &src, &out) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("lavrac: {e}");
                    ExitCode::from(1)
                }
            }
        }
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
                Err(f) => match f.code {
                    Some(code) if !f.desist => {
                        report(&Diagnostic::new(code, f.message, f.span));
                        ExitCode::from(101)
                    }
                    _ => {
                        report(&Diagnostic::failure(f.message, f.span).primary("o programa desistiu aqui"));
                        ExitCode::from(1)
                    }
                },
            }
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Gera o objeto, junta com o runtime e linka com o `cc` do sistema.
fn build(prog: &lavra::hir::Program, src: &SourceFile, out: &str) -> Result<(), String> {
    let object = lavra::codegen::compile(prog, &src.text, &src.name()).map_err(|e| format!("erro no gerador de código: {e}"))?;
    let dir = std::env::temp_dir().join(format!("lavrac-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let obj = dir.join("programa.o");
    let rt = dir.join("liblavra_rt.a");
    std::fs::write(&obj, object).map_err(|e| e.to_string())?;
    std::fs::write(&rt, RUNTIME).map_err(|e| e.to_string())?;
    let status = std::process::Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
        .arg("-o")
        .arg(out)
        .arg(&obj)
        .arg(&rt)
        .args(["-lgcc_s", "-lutil", "-lrt", "-lpthread", "-lm", "-ldl", "-lc"])
        .status()
        .map_err(|e| format!("não consegui chamar o linker (cc): {e}"))?;
    let _ = std::fs::remove_dir_all(&dir);
    if !status.success() {
        return Err("o linker (cc) falhou".into());
    }
    Ok(())
}
