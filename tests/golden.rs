//! Testes golden: cada `tests/programs/ok/X.lav` roda e a saída tem que ser igual a
//! `X.stdout` (com `X.stdin` como entrada, se existir); cada `tests/programs/err/X.lav`
//! tem que falhar com o diagnóstico igual a `X.stderr`.
//!
//! `LAVRA_BLESS=1 cargo test` regrava as expectativas a partir da saída atual.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn programs(dir: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "lav"))
        .collect();
    v.sort();
    v
}

fn run(path: &Path) -> (bool, String, String) {
    let rel = path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap();
    let stdin = std::fs::read(path.with_extension("stdin")).unwrap_or_default();
    let mut child = Command::new(env!("CARGO_BIN_EXE_lavrac"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["run", rel.to_str().unwrap()])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        let mut i = child.stdin.take().unwrap();
        let _ = i.write_all(&stdin);
    }
    let out = child.wait_with_output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

fn compare(path: &Path, ext: &str, actual: &str, failures: &mut Vec<String>) {
    let file = path.with_extension(ext);
    if std::env::var_os("LAVRA_BLESS").is_some() {
        std::fs::write(&file, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&file).unwrap_or_else(|_| panic!("falta {}", file.display()));
    if expected != actual {
        failures.push(format!("{}\n--- esperado\n{expected}\n--- obtido\n{actual}", file.display()));
    }
}

#[test]
fn ok_programs() {
    let mut failures = Vec::new();
    for p in programs("tests/programs/ok") {
        let (ok, out, err) = run(&p);
        if !ok && !p.with_extension("stderr").exists() {
            failures.push(format!("{} falhou:\n{err}", p.display()));
            continue;
        }
        compare(&p, "stdout", &out, &mut failures);
        if p.with_extension("stderr").exists() || !err.is_empty() {
            compare(&p, "stderr", &err, &mut failures);
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn err_programs() {
    let mut failures = Vec::new();
    for p in programs("tests/programs/err") {
        let (ok, _out, err) = run(&p);
        if ok {
            failures.push(format!("{} deveria falhar e passou", p.display()));
            continue;
        }
        compare(&p, "stderr", &err, &mut failures);
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
