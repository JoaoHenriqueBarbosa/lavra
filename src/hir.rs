//! Árvore resolvida e tipada: nomes viraram índices de locais, cada expressão tem tipo.

use crate::ast::BinOp;
use crate::span::Span;
use crate::types::Ty;

pub type LocalId = usize;

#[derive(Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub main: usize,
}

#[derive(Debug)]
pub struct Func {
    pub name: String,
    pub locals: Vec<Local>,
    pub body: Block,
    pub ret: Ty,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct Block(pub Vec<Stmt>);

#[derive(Debug)]
pub enum Stmt {
    Let { local: LocalId, value: Expr },
    Assign { place: Place, value: Expr },
    /// "cresce" e "diminui": `place op= value`.
    Compound { place: Place, op: BinOp, value: Expr, span: Span },
    Write { value: Expr, newline: bool },
}

#[derive(Debug, Clone)]
pub struct Place {
    pub root: LocalId,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Int(i128),
    Real(f64),
    Text(String),
    Char(char),
    Bool(bool),
    Read(Place),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Converte para texto e junta ("seguido de").
    Concat(Vec<Expr>),
}
