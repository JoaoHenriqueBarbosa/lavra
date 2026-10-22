//! Lavra: uma linguagem de programação compilada em que cada instrução é uma oração em
//! português, e a gramática é verificação.

pub mod ast;
pub mod codegen;
pub mod codes;
pub mod diagnostics;
pub mod explain;
pub mod hir;
pub mod interp;
pub mod lexer;
pub mod lexicon;
pub mod morphology;
pub mod ownership;
pub mod parser;
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
    Analysis { diagnostics: diags.sorted(), tokens, ast, hir, tables }
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
