//! As correções seguras (`lavrac fix` e o "corrigir tudo" do editor): sobre os programas
//! errados do corpus, aplicar tudo não quebra nada, aplicar de novo não muda nada, e o
//! programa cujos erros o compilador sabe corrigir passa a compilar.

use lavra::fix::fix_all;
use std::path::{Path, PathBuf};
use std::process::Command;

fn err_programs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs/err");
    let mut files: Vec<PathBuf> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|x| x == "lav")).collect();
    files.sort();
    files
}

#[test]
fn one_fix_reveals_the_next() {
    // O ponto final que falta esconde a concordância da linha de baixo.
    let src = "Para começar:\n    A nota é 7\n    Se a nota for maiores que 5: escrevo \"ok\".\n";
    let (fixed, applied) = fix_all(src);
    assert_eq!(fixed.text(), "Para começar:\n    A nota é 7.\n    Se a nota for maior que 5: escrevo \"ok\".\n");
    assert_eq!(applied, 2);
    // As edições são pequenas e sobre o texto original.
    assert!(fixed.edits().iter().all(|e| e.span.end - e.span.start <= 7), "{:?}", fixed.edits());
}

#[test]
fn the_err_corpus_fixes_what_it_can_and_only_that() {
    let mut failures = Vec::new();
    let mut compiled = 0;
    for path in err_programs() {
        let src = std::fs::read_to_string(&path).unwrap();
        let name = path.display().to_string();
        let before = lavra::analyze(&src);
        let (fixed, _) = fix_all(&src);
        let text = fixed.text();
        // Aplicar de novo não muda nada.
        let (again, _) = fix_all(&text);
        if !again.is_empty() {
            failures.push(format!("{name}: corrigir de novo ainda muda"));
        }
        // O que foi corrigido não volta: nenhum erro do código corrigido sobra. Erro que
        // estava escondido atrás da oração que não se lia pode aparecer, com outro código
        // ("As horas forem maiores que 10" revela que as horas são uma lista).
        let fixable: Vec<&str> =
            before.diagnostics.iter().filter(|d| d.is_error() && d.fixes.iter().any(|f| f.safe)).filter_map(|d| d.code).collect();
        let after = lavra::analyze(&text);
        if let Some(d) = after.diagnostics.iter().find(|d| d.is_error() && d.code.is_some_and(|c| fixable.contains(&c))) {
            failures.push(format!("{name}: o erro corrigido continua: {}", d.message));
        }
        let all_fixable = before.diagnostics.iter().filter(|d| d.is_error()).all(|d| d.fixes.iter().any(|f| f.safe));
        if all_fixable && !fixed.is_empty() && !after.has_errors() {
            compiled += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(compiled >= 10, "poucos programas corrigidos por inteiro: {compiled}");
}

#[test]
fn the_command_fixes_files_and_check_only_reports() {
    let dir = std::env::temp_dir().join(format!("lavra-fix-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("conta.lav");
    let src = "Para começar:\n    O saldo começa em 100\n    Escrevo o saldo.\n";
    std::fs::write(&file, src).unwrap();
    let lavrac = env!("CARGO_BIN_EXE_lavrac");
    let check = Command::new(lavrac).args(["fix", "--check"]).arg(&dir).output().unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&check.stdout).contains("1 correção a aplicar"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), src);
    let fix = Command::new(lavrac).arg("fix").arg(&file).output().unwrap();
    assert!(fix.status.success(), "{}", String::from_utf8_lossy(&fix.stderr));
    assert!(String::from_utf8_lossy(&fix.stdout).contains("(1 correção)"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "Para começar:\n    O saldo começa em 100.\n    Escrevo o saldo.\n");
    let again = Command::new(lavrac).args(["fix", "--check"]).arg(&file).output().unwrap();
    assert!(again.status.success());
    std::fs::remove_dir_all(&dir).unwrap();
}
