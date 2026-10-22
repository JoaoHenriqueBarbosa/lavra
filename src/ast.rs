//! Árvore sintática como foi escrita: cada nó guarda o que a gramática disse (artigo,
//! gênero, número, preposições), para que a análise semântica confira concordância,
//! escopo, tipos e posse.

use crate::morphology::{Gender, Number};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

/// Um nome como aparece no texto: com artigo definido ("a carta") ou símbolo ("x").
#[derive(Clone, Debug)]
pub struct NameRef {
    pub word: String,
    pub span: Span,
    /// Span só da palavra (sem o artigo).
    pub word_span: Span,
    pub article: Option<ArticleUse>,
}

impl NameRef {
    pub fn is_symbol(&self) -> bool {
        self.article.is_none()
    }

    pub fn number(&self) -> Number {
        self.article.map(|a| a.number).unwrap_or(Number::Singular)
    }

    pub fn gender(&self) -> Gender {
        self.article.map(|a| a.gender).unwrap_or(Gender::Masculine)
    }

    /// "a carta", ou "x".
    pub fn display(&self) -> String {
        match &self.article {
            Some(a) => format!("{} {}", crate::morphology::definite_article(a.gender, a.number), self.word),
            None => self.word.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArticleUse {
    pub gender: Gender,
    pub number: Number,
    pub span: Span,
    /// Forma contraída em que o artigo veio ("da"), se veio.
    pub contraction: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub enum TypeExpr {
    Int { bits: u8, signed: bool },
    Byte,
    Real { bits: u8 },
    Bool,
    Char,
    Text,
    List(Box<TypeExpr>),
    Slice(Box<TypeExpr>),
    Named(String),
    Maybe(Box<TypeExpr>),
}

/// Anotação de tipo: o aposto ("um inteiro de 32 bits") ou a tipagem de parâmetro.
#[derive(Clone, Debug)]
pub struct TypeAnn {
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamMode {
    /// Recebe a posse (move tipos donos, copia tipos de valor).
    Owned,
    /// "emprestado": empréstimo de leitura.
    Borrowed,
    /// "confiado": empréstimo exclusivo, pode modificar.
    Lent,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: NameRef,
    pub ty: TypeAnn,
    pub mode: ParamMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Sujeito de predicado ("Para o valor exceder ...").
    Subject,
    /// Objeto direto.
    Object,
    /// Complemento fixo: faz parte do nome do verbo ("contar os dias até").
    Fixed { article: String, word: String },
    /// Grupo regido por preposição; `count` é 1 ou 2 ("entre p e q").
    Prep { prep: String, count: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SigKind {
    Action,
    /// Predicado com verbo: "Para o valor exceder o limite".
    Predicate,
    /// Predicado adjetivo: "Para o número ser par"; guarda o adjetivo e o gênero da base.
    Adjective {
        adjective: String,
        base_gender: Gender,
    },
}

#[derive(Clone, Debug)]
pub struct FuncSig {
    pub infinitive: String,
    pub first_person: String,
    pub kind: SigKind,
    pub slots: Vec<Slot>,
    /// Parâmetros na ordem de chamada (sem os complementos fixos).
    pub params: Vec<Param>,
    pub ret: Option<TypeAnn>,
    pub fallible: bool,
    pub span: Span,
    pub name_span: Span,
    pub is_main: bool,
}

impl FuncSig {
    /// Forma de uso para mensagens: "transfiro QUANTIA de ORIGEM para DESTINO".
    pub fn usage(&self) -> String {
        let mut parts = Vec::new();
        let mut p = self.params.iter();
        let upper = |n: &NameRef| n.word.to_uppercase();
        match &self.kind {
            SigKind::Action => parts.push(self.first_person.clone()),
            SigKind::Predicate => {
                if let Some(s) = p.next() {
                    parts.push(upper(&s.name));
                }
                parts.push(crate::morphology::future_subjunctive(&self.infinitive, Number::Singular));
            }
            SigKind::Adjective { adjective, .. } => {
                if let Some(s) = p.next() {
                    parts.push(upper(&s.name));
                }
                parts.push(format!("for {adjective}"));
            }
        }
        for slot in &self.slots {
            match slot {
                Slot::Subject => {}
                Slot::Object => {
                    if let Some(x) = p.next() {
                        parts.push(upper(&x.name));
                    }
                }
                Slot::Fixed { article, word } => parts.push(format!("{article} {word}")),
                Slot::Prep { prep, count } => {
                    let names: Vec<String> = (0..*count).filter_map(|_| p.next()).map(|x| upper(&x.name)).collect();
                    parts.push(format!("{prep} {}", names.join(" e ")));
                }
            }
        }
        parts.join(" ")
    }
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub name: NameRef,
    pub ty: TypeAnn,
}

#[derive(Clone, Debug)]
pub struct RecordDef {
    pub word: String,
    pub plural: String,
    pub gender: Gender,
    pub span: Span,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
pub struct VariantDef {
    pub word: String,
    pub gender: Gender,
    pub span: Span,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
pub struct EnumDef {
    pub word: String,
    pub plural: String,
    pub gender: Gender,
    pub span: Span,
    pub variants: Vec<VariantDef>,
}

#[derive(Clone, Debug)]
pub struct FuncDef {
    pub sig: usize,
    pub body: Block,
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Period(Period),
    If { arms: Vec<(Cond, Block)>, otherwise: Option<Block>, span: Span },
    While { cond: Cond, body: Block, span: Span },
    Repeat { count: Option<Expr>, body: Block, span: Span },
    ForRange { var: NameRef, from: Expr, to: Expr, body: Block, span: Span },
    ForEach { item: NameRef, source: Expr, body: Block, span: Span },
    Match { scrutinee: Expr, arms: Vec<MatchArm>, otherwise: Option<Block>, span: Span },
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub patterns: Vec<Pattern>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Variant { word: String, gender: Gender, span: Span },
    Literal(Expr),
}

/// Período: orações coordenadas até o ponto final. "O resultado" vive no período.
#[derive(Clone, Debug)]
pub struct Period {
    pub clauses: Vec<Clause>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Init {
    Expr(Expr),
    /// Lista escrita por extenso ("7, 8,5 e 9"), só depois de sujeito plural.
    List(Vec<Expr>, Span),
    /// "começam vazias", "começa vazio".
    Empty(Span),
}

impl Init {
    pub fn span(&self) -> Span {
        match self {
            Init::Expr(e) => e.span,
            Init::List(_, s) | Init::Empty(s) => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Which {
    Ordinal { position: u32, span: Span },
    Item(Expr),
}

#[derive(Clone, Debug)]
pub struct Clause {
    pub kind: ClauseKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ClauseKind {
    Decl { name: NameRef, ann: Option<TypeAnn>, mutable: bool, init: Init },
    Assign { place: Place, value: Init },
    Grow { place: Place, value: Expr },
    Shrink { place: Place, value: Expr },
    Push { place: Place, value: Expr },
    Remove { place: Place, which: Which },
    Write { parts: Vec<Expr>, newline: bool },
    ReadInto { place: Place },
    ReadValue { ty: TypeAnn },
    Return(Option<Expr>),
    Desist(Expr),
    Break,
    Continue,
    Take { place: Place, which: Which },
    Call { call: Call, handler: Option<Fallback> },
}

#[derive(Clone, Debug)]
pub enum Fallback {
    Value(Box<Expr>),
    Desist(Option<Box<Expr>>),
}

#[derive(Clone, Debug)]
pub struct Call {
    pub sig: usize,
    pub args: Vec<Expr>,
    pub span: Span,
    pub verb_span: Span,
}

#[derive(Clone, Debug)]
pub enum Cond {
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Atom { negated: bool, kind: Box<CondKind>, span: Span },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rel {
    Greater,
    Less,
    Equal,
    Different,
    AtLeast,
    AtMost,
}

impl Rel {
    pub fn words(self) -> &'static str {
        match self {
            Rel::Greater => "maior que",
            Rel::Less => "menor que",
            Rel::Equal => "igual a",
            Rel::Different => "diferente de",
            Rel::AtLeast => "pelo menos",
            Rel::AtMost => "no máximo",
        }
    }
}

#[derive(Clone, Debug)]
pub enum CondKind {
    Rel { subject: Expr, rel: Rel, rhs: Expr },
    Variant { subject: Expr, word: String, gender: Gender, word_span: Span },
    Pred { call: Call },
    Exists { word: String, word_span: Span },
    Empty { subject: Expr },
    Contains { subject: Expr, item: Expr },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinOp {
    pub fn words(self) -> &'static str {
        match self {
            BinOp::Add => "mais",
            BinOp::Sub => "menos",
            BinOp::Mul => "vezes",
            BinOp::Div => "dividido por",
            BinOp::Rem => "módulo",
            BinOp::Pow => "a potência",
            BinOp::BitAnd => "a interseção",
            BinOp::BitOr => "a união",
            BinOp::BitXor => "a disjunção",
            BinOp::Shl => "o deslocamento à esquerda",
            BinOp::Shr => "o deslocamento à direita",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    BitNot,
    Sqrt,
    Abs,
    Len,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i128),
    Real(f64),
    Text(String),
    Char(char),
    Bool(bool),
    Nothing,
    Place(Place),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Unary { op: UnOp, operand: Box<Expr> },
    Call(Call),
    Anaphora,
    Fact(Box<Cond>),
    Copy(Box<Expr>),
    Convert { value: Box<Expr>, ty: TypeAnn },
    Construct { word: String, gender: Gender, word_span: Span, fields: Vec<(String, Span, Expr)> },
    Concat(Vec<Expr>),
    OrElse { value: Box<Expr>, fallback: Fallback },
    Slice { base: Box<Expr>, from: Box<Expr>, to: Box<Expr> },
}

#[derive(Clone, Debug)]
pub struct Place {
    pub root: NameRef,
    /// Projeções da raiz para fora: "o nome do titular da conta" = conta → titular → nome.
    pub proj: Vec<Proj>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Proj {
    Field(NameRef),
    Index(Box<Expr>, Span),
    Ordinal { position: u32, gender: Gender, span: Span },
}

impl Place {
    /// Número gramatical da cabeça (o que concorda com o verbo).
    pub fn number(&self) -> Number {
        match self.proj.last() {
            None => self.root.number(),
            Some(Proj::Field(n)) => n.number(),
            Some(_) => Number::Singular,
        }
    }

    pub fn head_gender(&self) -> Gender {
        match self.proj.last() {
            None => self.root.gender(),
            Some(Proj::Field(n)) => n.gender(),
            Some(Proj::Ordinal { gender, .. }) => *gender,
            Some(Proj::Index(..)) => Gender::Masculine,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub records: Vec<RecordDef>,
    pub enums: Vec<EnumDef>,
    pub sigs: Vec<FuncSig>,
    pub funcs: Vec<FuncDef>,
    /// Definições e orações do nível de fora.
    pub top: Vec<Stmt>,
    pub main: Option<usize>,
    /// Os nomes que o programa declara, como o parser os achou.
    pub vocab: Vocabulary,
}

/// O que o programa declara e o parser precisa conhecer antes de ler os corpos: os
/// verbos (pela 1ª pessoa e pelo infinitivo), os registros, as enumerações, os casos e os
/// campos. As pré-varreduras produzem; a leitura dos corpos consulta. Os índices apontam
/// para `Program::sigs`, `Program::records` e `Program::enums`.
///
/// É também o que uma biblioteca acrescenta ao programa que a usa, e o que o servidor de
/// linguagem oferece ao completar.
#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    /// Verbos de ação pela 1ª pessoa: "transfiro" → a assinatura.
    pub by_first_person: HashMap<String, usize>,
    /// Todas as assinaturas de um infinitivo (a mesma ação com regências diferentes).
    pub by_infinitive: HashMap<String, Vec<usize>>,
    pub records: HashMap<String, usize>,
    pub record_plurals: HashMap<String, usize>,
    pub enums: HashMap<String, usize>,
    pub enum_plurals: HashMap<String, usize>,
    /// Caso → (enumeração, posição do caso).
    pub variants: HashMap<String, (usize, usize)>,
    /// Nomes de campo de todos os registros: "a soma de" é operação por extenso só se
    /// nenhum registro tiver o campo "soma".
    pub field_names: HashSet<String>,
}
