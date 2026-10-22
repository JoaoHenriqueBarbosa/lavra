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
    for p in programs("tests/programs/ok").into_iter().chain(programs("samples")) {
        let (ok, out, err) = run(&p);
        if !ok && !p.with_extension("stderr").exists() && std::env::var_os("LAVRA_BLESS").is_none() {
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

/// Tira da saída de erro os avisos de compilação (o executável nativo não os tem).
fn without_warnings(stderr: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in stderr.split_inclusive('\n') {
        let starts_block = !line.starts_with(' ') && !line.starts_with(|c: char| c.is_ascii_digit()) && !line.trim().is_empty();
        if starts_block {
            skipping = line.starts_with("Aviso:");
        }
        if !skipping {
            out.push_str(line);
        }
    }
    out
}

/// Paridade: todo programa que compila roda igual no interpretador e no nativo
/// (saída, erro de execução e código de saída, byte a byte).
#[test]
fn native_parity() {
    let dir = std::env::temp_dir().join(format!("lavra-parity-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for p in programs("tests/programs/ok").into_iter().chain(programs("tests/programs/err")).chain(programs("samples")) {
        let rel = p.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap().to_str().unwrap().to_string();
        let exe = dir.join(p.file_stem().unwrap());
        let build = Command::new(env!("CARGO_BIN_EXE_lavrac"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["build", &rel, "-o", exe.to_str().unwrap()])
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        if !build.status.success() {
            // Programa com erro de compilação: não há o que comparar.
            continue;
        }
        checked += 1;
        let stdin = std::fs::read(p.with_extension("stdin")).unwrap_or_default();
        let run_with = |cmd: &mut Command| {
            let mut child = cmd.env("NO_COLOR", "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
            {
                use std::io::Write;
                let _ = child.stdin.take().unwrap().write_all(&stdin);
            }
            child.wait_with_output().unwrap()
        };
        let interp = run_with(Command::new(env!("CARGO_BIN_EXE_lavrac")).current_dir(env!("CARGO_MANIFEST_DIR")).args(["run", &rel]));
        let native = run_with(Command::new(&exe).current_dir(env!("CARGO_MANIFEST_DIR")));
        let i_err = without_warnings(&String::from_utf8_lossy(&interp.stderr));
        let n_err = String::from_utf8_lossy(&native.stderr).into_owned();
        if interp.stdout != native.stdout || i_err != n_err || interp.status.code() != native.status.code() {
            failures.push(format!(
                "{rel}\n--- interpretador ({:?})\n{}{}\n--- nativo ({:?})\n{}{}",
                interp.status.code(),
                String::from_utf8_lossy(&interp.stdout),
                i_err,
                native.status.code(),
                String::from_utf8_lossy(&native.stdout),
                n_err
            ));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(checked > 0);
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
