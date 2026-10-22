//! Lavra: uma linguagem de programação compilada em que cada instrução é uma oração em
//! português, e a gramática é verificação.

pub mod ast;
#[cfg(feature = "native")]
pub mod codegen;
pub mod codes;
#[cfg(feature = "dap")]
pub mod dap;
pub mod debug;
pub mod diagnostics;
pub mod explain;
pub mod fix;
pub mod format;
pub mod hir;
pub mod interp;
pub mod lexer;
pub mod lexicon;
#[cfg(feature = "lsp")]
pub mod lsp;
pub mod morphology;
pub mod ownership;
pub mod parser;
pub mod rename;
pub mod sema;
pub mod span;
pub mod tables;
pub mod textfmt;
pub mod types;

use diagnostics::{Diagnostic, Diagnostics};

/// Tudo o que a análise de um texto produz: os diagnósticos (erros e avisos, na ordem do
/// texto), os tokens, a AST e a HIR. Com erro, a HIR existe, mas só serve ao editor.
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub tokens: Vec<lexer::Token>,
    pub ast: ast::Program,
    pub hir: hir::Program,
    /// Cada nome declarado e usado, os escopos e o tipo de cada expressão.
    pub tables: tables::Tables,
}

impl Analysis {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Analisa o texto inteiro sem parar no primeiro erro: cada etapa relata e segue.
pub fn analyze(text: &str) -> Analysis {
    let mut diags = Diagnostics::default();
    let tokens = lexer::tokenize(text, &mut diags);
    let ast = parser::parse(tokens.clone(), &mut diags);
    let (hir, tables) = sema::check(&ast, &mut diags);
    ownership::check(&hir, text, &mut diags);
    if !diags.has_errors() {
        diags.list.extend(sema::unused_warnings(&tables));
    }
    let mut diagnostics = diags.sorted();
    for d in &mut diagnostics {
        keep_case(d, text);
    }
    Analysis { diagnostics, tokens, ast, hir, tables }
}

/// Uma correção que troca o começo de uma oração mantém a maiúscula ("O saldo" vira "A
/// reserva", não "a reserva"): quem monta a correção não vê o texto escrito.
fn keep_case(d: &mut Diagnostic, text: &str) {
    for e in d.fixes.iter_mut().flat_map(|f| f.edits.iter_mut()) {
        let written = text.get(e.span.start as usize..e.span.end as usize).unwrap_or_default();
        if written.chars().next().is_some_and(char::is_uppercase) && e.text.chars().next().is_some_and(char::is_lowercase) {
            let mut c = e.text.chars();
            e.text = c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
        }
    }
}

/// Compila o texto até a HIR, pronta para os back-ends, e devolve os avisos junto. Com
/// erro, devolve todos os diagnósticos (avisos inclusive), na ordem do texto.
pub fn compile(text: &str) -> Result<(hir::Program, Vec<Diagnostic>), Vec<Diagnostic>> {
    let a = analyze(text);
    if a.has_errors() {
        return Err(a.diagnostics);
    }
    Ok((a.hir, a.diagnostics))
}
