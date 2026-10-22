//! Compila o runtime (`runtime/lib.rs`) como biblioteca estática, com o `rustc` do
//! próprio toolchain, para o `lavrac` embutir e linkar em cada executável.

use std::path::PathBuf;
use std::process::Command;

fn main() {
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
