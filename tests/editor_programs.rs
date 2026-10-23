//! Os programas de teste da gramática tree-sitter (`editors/tree-sitter-lavra/test/programs`)
//! são Lavra válida. O teste de deriva da gramática confere que ela os parseia sem erro; aqui o
//! compilador confere que eles são programas certos, com o que exercitam de propósito: tabs,
//! CRLF, o fim do texto sem quebra de linha, maiúsculas em qualquer lugar.

use std::path::Path;
use std::process::Command;

#[test]
fn tree_sitter_test_programs_are_valid_lavra() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("editors/tree-sitter-lavra/test/programs");
    // O pacote publicado não leva editors/ (o `exclude` do Cargo.toml): só o repositório tem os
    // programas, e o teste de deriva da gramática falha se a pasta sumir dele.
    if !dir.is_dir() {
        eprintln!("sem {}: os programas da gramática só existem no repositório", dir.display());
        return;
    }
    let mut programs: Vec<_> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "lav")).collect();
    programs.sort();
    assert!(!programs.is_empty(), "nenhum programa em {}", dir.display());
    let refused: Vec<String> = programs
        .iter()
        .filter_map(|p| {
            let out = Command::new(env!("CARGO_BIN_EXE_lavrac")).arg("check").arg(p).env("NO_COLOR", "1").output().unwrap();
            (!out.status.success())
                .then(|| format!("{}:\n{}{}", p.display(), String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
        })
        .collect();
    assert!(refused.is_empty(), "programas de teste da gramática que o compilador recusa:\n{}", refused.join("\n"));
}
