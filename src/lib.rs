//! Lavra: uma linguagem de programação compilada em que cada instrução é uma oração em
//! português, e a gramática é verificação.

pub mod ast;
pub mod diagnostics;
pub mod hir;
pub mod interp;
pub mod lexer;
pub mod morphology;
pub mod parser;
pub mod sema;
pub mod span;
pub mod types;

use diagnostics::Diagnostic;

/// Compila o texto-fonte até a HIR; devolve os avisos junto.
pub fn compile(text: &str) -> Result<(hir::Program, Vec<Diagnostic>), Box<Diagnostic>> {
    let tokens = lexer::tokenize(text)?;
    let ast = parser::parse(tokens)?;
    sema::check(&ast)
}
