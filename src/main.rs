//! `lavrac`: compila e roda programas Lavra.

use lavra::diagnostics::{Diagnostic, Style, render};
use lavra::span::SourceFile;
use std::io::IsTerminal;
use std::process::ExitCode;

const HELP: &str = "\
lavrac: compilador da Lavra, a linguagem em que cada instrução é uma oração.

uso:
  lavrac <arquivo.lav>                  roda o programa (o mesmo que run)
  lavrac run <arquivo.lav>              interpreta o programa
  lavrac build <arquivo.lav> [-o saída] compila para um executável nativo
                                        (padrão: o nome do arquivo sem .lav)
  lavrac check <arquivo.lav> [--json]   só confere gramática, tipos e posse;
                                        com --json, os diagnósticos em JSON
  lavrac explain <código>               explica um erro, com exemplo errado e certo
                                        (ex.: lavrac explain L0334)
  lavrac lexicon --json                 exporta o léxico da língua
  lavrac --version                      a versão e o commit
  lavrac --help                         esta ajuda

Variáveis: NO_COLOR desliga as cores; LAVRA_ECO=1 repete a entrada lida.
";

/// O runtime, compilado pelo build.rs e embutido no compilador.
const RUNTIME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/liblavra_rt.a"));

/// O que a linha de comando pediu.
enum Command {
    Run { path: String },
    Build { path: String, output: Option<String> },
    Check { path: String, json: bool },
    Explain { code: String },
    Lexicon,
    Version,
    Help,
}

/// Lê os argumentos; o erro é a mensagem para quem chamou.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let rest = |from: usize| args.get(from..).unwrap_or_default();
    let one_path = |cmd: &str, xs: &[String]| -> Result<String, String> {
        match xs {
            [p] => Ok(p.clone()),
            [] => Err(format!("\"{cmd}\" pede o arquivo .lav")),
            _ => Err(format!("\"{cmd}\" recebe um arquivo só")),
        }
    };
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help" | "ajuda") => Ok(Command::Help),
        Some("--version" | "-V" | "version" | "versão") => Ok(Command::Version),
        Some("run") => Ok(Command::Run { path: one_path("run", rest(1))? }),
        Some("build") => {
            let mut path = None;
            let mut output = None;
            let mut xs = rest(1).iter();
            while let Some(x) = xs.next() {
                if x == "-o" {
                    output = Some(xs.next().ok_or("depois de -o vem o nome do executável")?.clone());
                } else if path.is_none() {
                    path = Some(x.clone());
                } else {
                    return Err("\"build\" recebe um arquivo só".into());
                }
            }
            Ok(Command::Build { path: path.ok_or("\"build\" pede o arquivo .lav")?, output })
        }
        Some("check") => {
            let json = rest(1).iter().any(|x| x == "--json");
            let files: Vec<String> = rest(1).iter().filter(|x| *x != "--json").cloned().collect();
            Ok(Command::Check { path: one_path("check", &files)?, json })
        }
        Some("explain") => match rest(1) {
            [code] => Ok(Command::Explain { code: code.clone() }),
            _ => Err("\"explain\" pede um código, como L0334".into()),
        },
        Some("lexicon") => match rest(1) {
            [flag] if flag == "--json" => Ok(Command::Lexicon),
            _ => Err("\"lexicon\" só exporta em JSON: lavrac lexicon --json".into()),
        },
        Some(p) if p.ends_with(".lav") && args.len() == 1 => Ok(Command::Run { path: p.to_string() }),
        Some(other) => Err(format!("comando desconhecido: \"{other}\"")),
    }
}

/// Um defeito do compilador (pânico) não é erro do programa: diz isso e pede o relato.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("lavrac: isto é um defeito do compilador, não do seu programa.");
        eprintln!("{info}");
        eprintln!(
            "Abra uma issue em https://github.com/JoaoHenriqueBarbosa/lavra/issues com o programa que causou o defeito \
             e a versão (lavrac --version: {} {}).",
            env!("CARGO_PKG_VERSION"),
            env!("LAVRA_COMMIT")
        );
    }));
}

fn read_source(path: &str) -> Result<SourceFile, ExitCode> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(SourceFile::new(path, t)),
        Err(e) => {
            eprintln!("lavrac: não consegui ler {path}: {e}");
            Err(ExitCode::from(2))
        }
    }
}

fn main() -> ExitCode {
    install_panic_hook();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match parse_args(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("lavrac: {msg}.\n");
            eprint!("{HELP}");
            return ExitCode::from(2);
        }
    };
    let (path, action) = match cmd {
        Command::Help => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Command::Version => {
            println!("lavrac {} ({})", env!("CARGO_PKG_VERSION"), env!("LAVRA_COMMIT"));
            return ExitCode::SUCCESS;
        }
        Command::Lexicon => {
            print!("{}", lavra::lexicon::to_json());
            return ExitCode::SUCCESS;
        }
        Command::Explain { code } => {
            return match lavra::explain::text(&code) {
                Some(t) => {
                    print!("{t}");
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("lavrac: não há explicação para \"{code}\" (os códigos vão de L0001 em diante; veja o cabeçalho do erro)");
                    ExitCode::from(2)
                }
            };
        }
        Command::Check { path, json: true } => {
            let src = match read_source(&path) {
                Ok(s) => s,
                Err(code) => return code,
            };
            let analysis = lavra::analyze(&src.text);
            println!("{}", json_report(&analysis.diagnostics, &src));
            return if analysis.has_errors() { ExitCode::from(1) } else { ExitCode::SUCCESS };
        }
        Command::Check { path, json: false } => (path, Action::Check),
        Command::Run { path } => (path, Action::Run),
        Command::Build { path, output } => {
            let out = output.unwrap_or_else(|| path.strip_suffix(".lav").unwrap_or(&path).to_string());
            (path, Action::Build(out))
        }
    };
    let src = match read_source(&path) {
        Ok(s) => s,
        Err(code) => return code,
    };
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
    match action {
        Action::Check => ExitCode::SUCCESS,
        Action::Build(out) => match build(&prog, &src, &out) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("lavrac: {e}");
                ExitCode::from(1)
            }
        },
        Action::Run => {
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
    }
}

/// O que fazer com o programa depois de compilado.
enum Action {
    Check,
    Run,
    Build(String),
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

/// Os diagnósticos em JSON, para editores e ferramentas. Linhas e colunas começam em 1;
/// a coluna conta caracteres, e `offset` é o byte no texto.
fn json_report(diags: &[Diagnostic], src: &SourceFile) -> String {
    use serde_json::{Value, json};
    let pos = |offset: u32| {
        let (line, column) = src.line_col(offset);
        json!({ "line": line, "column": column, "offset": offset })
    };
    let range = |s: lavra::span::Span| json!({ "start": pos(s.start), "end": pos(s.end) });
    let list: Vec<Value> = diags
        .iter()
        .map(|d| {
            json!({
                "code": d.code,
                "title": d.code.and_then(lavra::codes::find).map(|c| c.title),
                "category": d.category.id(),
                "severity": if d.is_error() { "error" } else { "warning" },
                "message": d.message,
                "range": range(d.span),
                "primary": d.primary,
                "labels": d.labels.iter().map(|l| json!({ "range": range(l.span), "text": l.text })).collect::<Vec<_>>(),
                "notes": d.notes,
                "hint": d.hint,
                "fixes": d.fixes.iter().map(|f| json!({
                    "title": f.title,
                    "safe": f.safe,
                    "edits": f.edits.iter().map(|e| json!({ "range": range(e.span), "text": e.text })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let errors = diags.iter().filter(|d| d.is_error()).count();
    serde_json::to_string_pretty(&json!({
        "file": src.name(),
        "errors": errors,
        "warnings": diags.len() - errors,
        "diagnostics": list,
    }))
    .expect("diagnósticos viram JSON")
}
