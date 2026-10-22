//! Compila o runtime (`runtime/lib.rs`) como biblioteca estática, com o `rustc` do
//! próprio toolchain, para o `lavrac` embutir e linkar em cada executável.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    commit();
    explain_table();
    // Sem o back-end nativo (o playground em wasm32), o runtime não entra.
    if std::env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }
    for f in ["runtime/lib.rs", "src/span.rs", "src/codes.rs", "src/diagnostics.rs", "src/textfmt.rs"] {
        println!("cargo:rerun-if-changed={f}");
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("liblavra_rt.a");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let status = Command::new(rustc)
        .args([
            "--edition",
            "2024",
            "--crate-type",
            "staticlib",
            "--crate-name",
            "lavra_rt",
            "-C",
            "panic=abort",
            "-C",
            "opt-level=2",
            "-C",
            "lto=fat",
            "-C",
            "codegen-units=1",
            "-C",
            "strip=debuginfo",
            "runtime/lib.rs",
            "-o",
        ])
        .arg(&out)
        .status()
        .expect("rustc para o runtime");
    assert!(status.success(), "o runtime não compilou");
}

/// O commit de onde o compilador foi construído, para o `lavrac --version`.
fn commit() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    let hash = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "commit desconhecido".into());
    println!("cargo:rustc-env=LAVRA_COMMIT={hash}");
}

/// A tabela dos textos de `lavrac explain`: um `include_str!` por `src/explain/LXXXX.md`.
fn explain_table() {
    println!("cargo:rerun-if-changed=src/explain");
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/explain");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "md")).collect())
        .unwrap_or_default();
    files.sort();
    let mut out = String::from("/// (código, texto) de cada explicação, gerado pelo build.rs.\npub static TEXTS: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let id = f.file_stem().unwrap().to_string_lossy();
        out.push_str(&format!("    ({id:?}, include_str!({:?})),\n", f.display().to_string()));
    }
    out.push_str("];\n");
    let path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("explain.rs");
    std::fs::write(path, out).unwrap();
}
