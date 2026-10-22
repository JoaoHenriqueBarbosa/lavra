//! Árvore resolvida e tipada: nomes viraram índices de locais e globais, cada
//! expressão tem tipo, cada chamada aponta para a função.

use crate::ast::{BinOp, ParamMode, Rel, UnOp};
use crate::span::Span;
use crate::types::Ty;

pub type LocalId = usize;
pub type GlobalId = usize;
pub type FuncId = usize;

#[derive(Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub globals: Vec<Global>,
    /// Inicializa os globais, na ordem em que foram escritos.
    pub init: Func,
    pub main: FuncId,
}

#[derive(Debug)]
pub struct Global {
    pub name: String,
    pub display: String,
    pub ty: Ty,
}

#[derive(Debug)]
pub struct Func {
    pub name: String,
    pub params: Vec<(LocalId, ParamMode)>,
    pub locals: Vec<Local>,
    pub body: Block,
    pub ret: Ty,
    /// "salvo uma falha": devolve, além do valor, a falha (ou nada).
    pub fallible: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    /// Com artigo, para as mensagens: "a carta".
    pub display: String,
    pub ty: Ty,
    pub mutable: bool,
    /// Vista emprestada de outro valor (item de "Para cada", valor de "Se houver"):
    /// lê-se, mas não se entrega.
    pub borrowed: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Var {
    Local(LocalId),
    Global(GlobalId),
}

#[derive(Debug, Default)]
pub struct Block(pub Vec<Stmt>);

#[derive(Debug)]
pub enum Stmt {
    Let {
        var: Var,
        value: Expr,
    },
    Assign {
        place: Place,
        value: Expr,
    },
    /// "cresce" e "diminui": `place op= value`.
    Compound {
        place: Place,
        op: BinOp,
        value: Expr,
        span: Span,
    },
    Write {
        value: Expr,
        newline: bool,
    },
    If {
        arms: Vec<(Cond, Block)>,
        otherwise: Option<Block>,
    },
    While {
        cond: Cond,
        body: Block,
    },
    /// "Repito N vezes" (com contagem) ou "Repito sempre" (sem).
    Repeat {
        count: Option<Expr>,
        body: Block,
    },
    /// "Para cada i de A até B": conta para baixo se A for maior que B.
    ForRange {
        var: LocalId,
        from: Expr,
        to: Expr,
        body: Block,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    /// Chamada usada como oração.
    Expr(Expr),
    /// "A lista ganha 7."
    Push {
        place: Place,
        value: Expr,
    },
    /// "Para cada nota das notas": a coleção fica emprestada ao laço.
    ForEach {
        var: LocalId,
        source: Expr,
        body: Block,
    },
    /// "Desisto com ...": a função falha com a mensagem.
    Desist(Expr),
    /// "Conforme a figura: Sendo um círculo: ...".
    Match { scrutinee: Expr, arms: Vec<Arm>, otherwise: Option<Block> },
}

#[derive(Debug)]
pub struct Arm {
    /// Os casos que levam a este braço.
    pub tags: Vec<usize>,
    /// Campos do caso vistos como nomes dentro do braço: (local, campo).
    pub binds: Vec<(LocalId, usize)>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub enum Cond {
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Not(Box<Cond>),
    Cmp {
        rel: Rel,
        lhs: Expr,
        rhs: Expr,
    },
    /// Expressão lógica: predicado chamado.
    Test(Expr),
    /// "Se houver apelido".
    IsSome(Expr),
    /// "Se a figura for um círculo".
    IsVariant(Expr, usize),
}

#[derive(Debug, Clone)]
pub struct Place {
    pub root: Var,
    /// Da raiz para fora: "o nome do titular da conta" = conta, titular, nome.
    pub proj: Vec<PlaceProj>,
    pub ty: Ty,
    pub span: Span,
}

impl Place {
    pub fn var(root: Var, ty: Ty, span: Span) -> Place {
        Place { root, proj: Vec::new(), ty, span }
    }
}

#[derive(Debug, Clone)]
pub enum PlaceProj {
    Field(usize),
    /// Posição a partir de 1 ("o item 2", "a segunda").
    Index(Box<Expr>, Span),
    /// "o último", "a última".
    Last(Span),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub value: Expr,
    /// O parâmetro que recebe, com artigo ("a origem").
    pub param: String,
    pub mode: ParamMode,
    /// Para parâmetro confiado: o lugar do chamador que recebe o valor de volta.
    pub place: Option<Place>,
}

#[derive(Debug, Clone)]
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
    /// Lê uma linha da entrada e converte para o tipo da expressão.
    Input,
    Call {
        func: FuncId,
        args: Vec<Arg>,
    },
    Fact(Box<Cond>),
    Unary(UnOp, Box<Expr>),
    List(Vec<Expr>),
    /// Valores dos campos, na ordem da definição.
    Record(Vec<Expr>),
    Copy(Box<Expr>),
    /// "Tomo a última das notas": tira o item da coleção e devolve.
    Take {
        place: Place,
        index: Option<Box<Expr>>,
    },
    Nothing,
    /// Um valor posto onde se espera "talvez".
    Wrap(Box<Expr>),
    /// O valor de dentro de um "talvez" que se sabe presente.
    Unwrap(Box<Expr>),
    /// Chamada que pode falhar: com valor de reserva, ou passando a falha adiante (`None`).
    Try {
        call: Box<Expr>,
        fallback: Option<Box<Expr>>,
    },
    /// Valor de enumeração: o caso e os campos.
    Variant { tag: usize, fields: Vec<Expr> },
    /// Campo do caso já conhecido (dentro de um braço de "Conforme").
    VariantField { value: Box<Expr>, field: usize },
}
