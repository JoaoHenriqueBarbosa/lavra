//! As tabelas de símbolos de uma análise: cada nome declarado (`Def`), cada uso dele
//! (`Ref`), os escopos, o tipo de cada expressão e o modo de cada argumento. Saem das
//! próprias passadas do compilador (a sema registra onde declara e onde resolve, a posse
//! registra onde cada valor é entregue), nunca de uma segunda resolução de nomes: o que o
//! editor mostra é o que o compilador entendeu.

use crate::ast::{ParamMode, VerbForm};
use crate::morphology::{Gender, Number};
use crate::span::Span;
use crate::types::Ty;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefKind {
    Function,
    Param,
    Local,
    Constant,
    Record,
    Field,
    Enum,
    Variant,
    /// "o resultado" de uma oração: a língua cria, o programa não nomeia.
    Result,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub kind: DefKind,
    /// A forma de dicionário: o infinitivo de uma função, a palavra de um nome.
    pub name: String,
    /// Com artigo ("a carta"); numa função, a regência ("transfiro QUANTIA de ORIGEM").
    pub display: String,
    pub gender: Gender,
    pub number: Number,
    pub ty: Option<Ty>,
    /// Onde foi declarado: o nome (com o artigo) ou o verbo do cabeçalho.
    pub span: Span,
    /// O escopo em que o nome vale (0 é o programa inteiro).
    pub scope: usize,
    pub mutable: bool,
    pub mode: Option<ParamMode>,
    /// Quem contém: a função de um parâmetro, o registro ou caso de um campo, a
    /// enumeração de um caso.
    pub owner: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Read,
    Write,
    Call,
    /// O nome reaparece na própria declaração (a tipagem do cabeçalho: "sendo a quantia
    /// um inteiro"); não conta como uso.
    Mention,
}

/// Como o nome foi escrito num uso: quem renomeia reescreve cada uso na mesma forma.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Como foi declarado: com artigo (ou contração), ou como símbolo; um tipo, no
    /// singular.
    Name,
    /// O singular tirado do plural: "Para cada nota" acha "as notas".
    Singular,
    /// Um tipo no plural: "uma lista de contas".
    Plural,
    /// Uma forma do verbo de uma função.
    Verb(VerbForm),
}

#[derive(Clone, Copy, Debug)]
pub struct Ref {
    pub def: usize,
    pub span: Span,
    pub use_: Use,
    pub form: Form,
}

/// O que a posse diz de um trecho: o nome entrega o valor ali (e dali em diante não se
/// usa), ou ganha um valor novo depois de ter entregado o dele.
#[derive(Clone, Debug)]
pub struct Ownership {
    pub span: Span,
    /// O valor sai das mãos do nome aqui.
    pub moved: bool,
    /// A frase do hover: "A carta é entregue a \"arquivar\" aqui: daqui em diante, não se
    /// usa mais."
    pub note: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Scope {
    pub parent: Option<usize>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct Tables {
    pub defs: Vec<Def>,
    pub refs: Vec<Ref>,
    pub scopes: Vec<Scope>,
    /// (trecho, tipo) de cada expressão; para o mesmo trecho vale a primeira, a de fora.
    pub expr_types: Vec<(Span, Ty)>,
    /// O modo de cada argumento de chamada: dono, emprestado ou confiado.
    pub args: Vec<(Span, ParamMode)>,
    /// Onde cada valor é entregue, e onde o nome volta a ter um; na ordem do texto.
    pub ownership: Vec<Ownership>,
}

fn contains(span: Span, offset: u32) -> bool {
    span.start <= offset && offset <= span.end && !(span.is_empty() && span.start != offset)
}

impl Tables {
    /// Junta os usos do mesmo nome no mesmo trecho (o argumento confiado é lido e
    /// mudado no mesmo lugar): fica um, e a escrita prevalece sobre a leitura.
    pub fn dedupe(&mut self) {
        let mut seen: std::collections::HashMap<(usize, u32, u32), usize> = std::collections::HashMap::new();
        let mut out: Vec<Ref> = Vec::with_capacity(self.refs.len());
        for r in self.refs.drain(..) {
            match seen.get(&(r.def, r.span.start, r.span.end)) {
                Some(&i) => {
                    if r.use_ == Use::Write {
                        out[i].use_ = Use::Write;
                    }
                }
                None => {
                    seen.insert((r.def, r.span.start, r.span.end), out.len());
                    out.push(r);
                }
            }
        }
        self.refs = out;
    }

    /// O nome declarado ou usado na posição: o uso mais curto que a contém, ou a
    /// declaração.
    pub fn def_at(&self, offset: u32) -> Option<usize> {
        let by_ref = self.refs.iter().filter(|r| contains(r.span, offset)).min_by_key(|r| r.span.end - r.span.start).map(|r| r.def);
        by_ref.or_else(|| {
            self.defs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.kind != DefKind::Result && contains(d.span, offset))
                .min_by_key(|(_, d)| d.span.end - d.span.start)
                .map(|(i, _)| i)
        })
    }

    /// Os usos de um nome, na ordem do texto.
    pub fn refs_of(&self, def: usize) -> Vec<Ref> {
        let mut v: Vec<Ref> = self.refs.iter().filter(|r| r.def == def).copied().collect();
        v.sort_by_key(|r| r.span.start);
        v
    }

    /// O tipo da menor expressão que contém a posição.
    pub fn type_at(&self, offset: u32) -> Option<(Span, &Ty)> {
        self.expr_types.iter().filter(|(s, _)| contains(*s, offset)).min_by_key(|(s, _)| s.end - s.start).map(|(s, t)| (*s, t))
    }

    /// O que a posse diz do menor trecho que contém a posição.
    pub fn ownership_at(&self, offset: u32) -> Option<&Ownership> {
        self.ownership.iter().filter(|o| contains(o.span, offset)).min_by_key(|o| o.span.end - o.span.start)
    }

    /// O escopo mais interno que contém a posição.
    pub fn scope_at(&self, offset: u32) -> usize {
        self.scopes
            .iter()
            .enumerate()
            .filter(|(_, s)| contains(s.span, offset))
            .min_by_key(|(_, s)| s.span.end - s.span.start)
            .map_or(0, |(i, _)| i)
    }

    /// Os nomes que valem na posição: os do escopo dela e dos de fora, declarados antes
    /// (funções, tipos e constantes valem no programa inteiro).
    pub fn visible_at(&self, offset: u32) -> Vec<usize> {
        let mut chain = vec![self.scope_at(offset)];
        while let Some(p) = self.scopes.get(*chain.last().unwrap()).and_then(|s| s.parent) {
            chain.push(p);
        }
        self.defs
            .iter()
            .enumerate()
            .filter(|(_, d)| d.kind != DefKind::Result && chain.contains(&d.scope))
            .filter(|(_, d)| d.scope == 0 || d.span.start <= offset)
            .map(|(i, _)| i)
            .collect()
    }
}
