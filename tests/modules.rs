//! Os arquivos trazidos ("Uso o arquivo ..."): o programa de `tests/modules` no
//! interpretador e no nativo, os arquivos rodando sozinhos, e os erros de quem pede errado.

use std::path::Path;
use std::process::Command;

const LAVRAC: &str = env!("CARGO_BIN_EXE_lavrac");

/// Roda o `lavrac` na raiz do repositório; devolve a saída, os erros e o código.
fn lavrac(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(LAVRAC).args(args).env("NO_COLOR", "1").current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("lavrac");
    (String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}

const PROGRAM_OUT: &str = "ANA: 105\n0,05\n9\numa conta com o titular Ana e o saldo 105\n";

#[test]
fn the_program_runs_with_what_the_files_define() {
    let (out, err, code) = lavrac(&["run", "tests/modules/programa.lav"]);
    assert_eq!(out, PROGRAM_OUT);
    // A falha declarada num arquivo trazido se mostra nele.
    assert!(err.contains("Falha: saldo insuficiente") && err.contains("--> tests/modules/lib/contas.lav:26:62"), "{err}");
    assert_eq!(code, 1);
}

#[test]
fn the_native_program_says_the_same() {
    let exe = std::env::temp_dir().join(format!("lavra-modules-{}", std::process::id()));
    let (_, err, code) = lavrac(&["build", "tests/modules/programa.lav", "-o", exe.to_str().unwrap()]);
    assert_eq!(code, 0, "{err}");
    let native = Command::new(&exe).env("NO_COLOR", "1").current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("o executável");
    let _ = std::fs::remove_file(&exe);
    let (out, err, code) = lavrac(&["run", "tests/modules/programa.lav"]);
    assert_eq!(String::from_utf8_lossy(&native.stdout), out);
    assert_eq!(String::from_utf8_lossy(&native.stderr), err);
    assert_eq!(native.status.code(), Some(code));
}

#[test]
fn a_file_with_its_own_start_runs_alone() {
    let (out, _, code) = lavrac(&["run", "tests/modules/lib/contas.lav"]);
    assert_eq!(out, "contas.lav rodando sozinho\nTESTE: 10\n4\n");
    assert_eq!(code, 0);
}

#[test]
fn a_file_of_definitions_checks_but_does_not_run() {
    let (_, err, code) = lavrac(&["check", "tests/modules/lib/formas.lav"]);
    assert_eq!((err.as_str(), code), ("", 0));
    let (_, err, code) = lavrac(&["run", "tests/modules/lib/formas.lav"]);
    assert!(err.contains("[L0300]") && err.contains("não roda sozinho"), "{err}");
    assert_eq!(code, 1);
}

#[test]
fn wrong_requests_and_errors_in_the_files() {
    let (_, err, code) = lavrac(&["check", "tests/modules/erros/pedidos.lav"]);
    assert_eq!(code, 1);
    // O arquivo que não existe e o pedido fora da forma, no arquivo do programa.
    assert!(err.contains("[L0178]: Não consegui trazer o arquivo \"nao-existe.lav\": não existe."), "{err}");
    assert!(err.contains("[L0177]") && err.contains("--> tests/modules/erros/pedidos.lav:2:1"), "{err}");
    // O erro de dentro de um arquivo trazido, no arquivo dele.
    assert!(err.contains("[L0359]") && err.contains("--> tests/modules/erros/quebrado.lav:2:27"), "{err}");
    // O verbo definido nos dois lados, com o lugar do outro.
    assert!(err.contains("\"dobrar\" já foi definido") && err.contains("definido aqui: tests/modules/erros/quebrado.lav:1"), "{err}");
}

#[test]
fn the_editor_sees_the_error_at_the_request() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modules/erros/pedidos.lav");
    let text = std::fs::read_to_string(&path).unwrap();
    let a = lavra::analyze_in(&text, Some(&path), &|p| std::fs::read_to_string(p));
    let inside: Vec<_> = a.diagnostics.iter().filter(|d| lavra::modules::is_module(d.span.start)).collect();
    assert!(!inside.is_empty());
    for d in inside {
        let shown = a.sources.at_request(d).expect("o pedido que trouxe o arquivo");
        assert!(shown.message.starts_with("No arquivo \""), "{}", shown.message);
        assert!(text[shown.span.start as usize..shown.span.end as usize].starts_with("Uso o arquivo"), "{:?}", shown.span);
    }
    // Sem quem leia arquivos (o navegador), o pedido é um erro, e não um pânico.
    let a = lavra::analyze_in(&text, None, &|_| Err(std::io::Error::from(std::io::ErrorKind::Unsupported)));
    assert!(a.diagnostics.iter().any(|d| d.code == Some("L0178")));
}
