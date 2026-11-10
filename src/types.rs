//! Tipos da Lavra depois da análise, e como se dizem em português.

use std::ops::Deref;
use std::sync::{Arc, OnceLock};

/// Um valor posto uma vez, depois de criado o dono. Os campos de um registro e os casos de
/// uma enumeração entram assim: o tipo já existe (e pode ser citado) enquanto os campos
/// dele se montam, que é o que deixa um registro guardar uma lista de si mesmo.
pub struct Late<T>(OnceLock<T>);

impl<T> Late<T> {
    pub fn empty() -> Self {
        Late(OnceLock::new())
    }

    pub fn of(value: T) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(value);
        Late(cell)
    }

    pub fn set(&self, value: T) {
        let _ = self.0.set(value);
    }
}

impl<T> Deref for Late<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.get().expect("os campos do tipo são postos antes de alguém os ler")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IntTy {
    pub bits: u8,
    pub signed: bool,
}

impl IntTy {
    pub const I64: IntTy = IntTy { bits: 64, signed: true };

    pub fn min(self) -> i128 {
        if self.signed { -(1i128 << (self.bits - 1)) } else { 0 }
    }

    pub fn max(self) -> i128 {
        if self.signed { (1i128 << (self.bits - 1)) - 1 } else { (1i128 << self.bits) - 1 }
    }

    pub fn contains(self, v: i128) -> bool {
        v >= self.min() && v <= self.max()
    }

    /// "inteiro", "natural de 32 bits", "byte".
    pub fn noun(self, plural: bool) -> String {
        if !self.signed && self.bits == 8 {
            return if plural { "bytes" } else { "byte" }.to_string();
        }
        let base = match (self.signed, plural) {
            (true, false) => "inteiro",
            (true, true) => "inteiros",
            (false, false) => "natural",
            (false, true) => "naturais",
        };
        if self.bits == 64 { base.to_string() } else { format!("{base} de {} bits", self.bits) }
    }
}

/// Um tipo registro definido pelo usuário ("Uma conta é um registro de: ...").
pub struct RecordTy {
    pub id: usize,
    pub name: String,
    pub plural: String,
    pub feminine: bool,
    pub fields: Late<Vec<(String, Ty)>>,
    /// Artigo de cada campo ("o saldo", "a origem"), para escrever o registro.
    pub field_articles: Vec<(String, &'static str)>,
}

/// Um caso de enumeração ("um círculo, com o raio, um real").
#[derive(Debug, PartialEq)]
pub struct VariantTy {
    pub name: String,
    pub feminine: bool,
    pub fields: Vec<(String, Ty)>,
    pub field_articles: Vec<(String, &'static str)>,
}

/// Uma enumeração ("Uma forma é uma destas: ...").
pub struct EnumTy {
    pub id: usize,
    pub name: String,
    pub plural: String,
    pub feminine: bool,
    pub variants: Late<Vec<VariantTy>>,
}

// Um tipo do programa é o nome dele: comparar e mostrar pelos campos não terminaria num
// tipo que cita a si mesmo.
impl PartialEq for RecordTy {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name
    }
}

impl PartialEq for EnumTy {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name
    }
}

impl std::fmt::Debug for RecordTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RecordTy({})", self.name)
    }
}

impl std::fmt::Debug for EnumTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EnumTy({})", self.name)
    }
}

/// Um tipo qualquer de uma função genérica ("uma coisa qualquer"). Só existe enquanto a
/// função é conferida: cada chamada troca o tipo qualquer pelo tipo de quem chama, e os
/// back-ends nunca o veem.
#[derive(Debug)]
pub struct ParamTy {
    /// A assinatura que o declara e a posição entre os tipos quaisquer dela.
    pub sig: usize,
    pub index: usize,
    pub name: String,
    pub feminine: bool,
    /// "qualquer ordenável": compara-se com "maior que".
    pub ordered: bool,
    /// "que se descreve": os verbos (no infinitivo) que o tipo tem de ter.
    pub verbs: Vec<String>,
}

impl PartialEq for ParamTy {
    fn eq(&self, other: &Self) -> bool {
        self.sig == other.sig && self.index == other.index
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int(IntTy),
    Real(u8),
    Bool,
    Char,
    Text,
    List(Box<Ty>),
    /// "um conjunto de textos": sem repetição, em ordem.
    Set(Box<Ty>),
    /// "um mapa de textos para inteiros": a chave e o valor.
    Map(Box<Ty>, Box<Ty>),
    Record(Arc<RecordTy>),
    Enum(Arc<EnumTy>),
    /// "talvez um texto": um valor ou nada.
    Maybe(Box<Ty>),
    /// Um tipo qualquer de uma função genérica.
    Param(Arc<ParamTy>),
    /// "um verbo de inteiro para inteiro": um verbo do programa como valor. De cada valor
    /// que recebe, o tipo e se vem emprestado; e o que devolve.
    Verb(Vec<(Ty, bool)>, Box<Ty>),
    Unit,
    /// Ainda por inferir (lista vazia antes do primeiro uso).
    Infer(u32),
    /// Tipo de uma expressão que já deu erro (não gera erros em cascata).
    Error,
}

impl Ty {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int(_) | Ty::Real(_) | Ty::Error)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Tipos de valor se copiam; os donos (texto, lista, registro) se movem.
    pub fn is_copy(&self) -> bool {
        // "talvez" é dono mesmo de um inteiro: no nativo é um ponteiro que pode ser nulo.
        matches!(self, Ty::Int(_) | Ty::Real(_) | Ty::Bool | Ty::Char | Ty::Unit | Ty::Error | Ty::Verb(..))
    }

    /// Lista, conjunto ou mapa: o que um nome no plural guarda.
    pub fn is_collection(&self) -> bool {
        matches!(self, Ty::List(_) | Ty::Set(_) | Ty::Map(..))
    }

    /// Serve de chave de mapa e de valor de conjunto: tem ordem total e não é dono de nada
    /// além de si.
    pub fn is_key(&self) -> bool {
        matches!(self, Ty::Int(_) | Ty::Char | Ty::Text | Ty::Bool | Ty::Error)
    }

    /// O tipo tem ordem: compara-se com "maior que" e "menor que".
    pub fn is_ordered(&self) -> bool {
        match self {
            Ty::Int(_) | Ty::Real(_) | Ty::Char | Ty::Text | Ty::Error => true,
            Ty::Param(p) => p.ordered,
            _ => false,
        }
    }

    /// O tipo cita um tipo qualquer: ainda não é o de uma chamada.
    pub fn has_param(&self) -> bool {
        match self {
            Ty::Param(_) => true,
            Ty::List(t) | Ty::Set(t) | Ty::Maybe(t) => t.has_param(),
            Ty::Map(k, v) => k.has_param() || v.has_param(),
            Ty::Verb(params, ret) => ret.has_param() || params.iter().any(|(t, _)| t.has_param()),
            _ => false,
        }
    }

    /// O tipo com cada tipo qualquer da assinatura `sig` trocado pelo de `args`.
    pub fn with_params(&self, sig: usize, args: &[Ty]) -> Ty {
        match self {
            // Sem tipos de chamada (a função conferida como foi escrita), o tipo qualquer fica.
            Ty::Param(p) if p.sig == sig => args.get(p.index).cloned().unwrap_or_else(|| self.clone()),
            Ty::List(t) => Ty::List(Box::new(t.with_params(sig, args))),
            Ty::Set(t) => Ty::Set(Box::new(t.with_params(sig, args))),
            Ty::Maybe(t) => Ty::Maybe(Box::new(t.with_params(sig, args))),
            Ty::Map(k, v) => Ty::Map(Box::new(k.with_params(sig, args)), Box::new(v.with_params(sig, args))),
            Ty::Verb(params, ret) => {
                Ty::Verb(params.iter().map(|(t, b)| (t.with_params(sig, args), *b)).collect(), Box::new(ret.with_params(sig, args)))
            }
            other => other.clone(),
        }
    }

    /// O tipo é um verbo, ou leva um dentro: não se escreve nem se compara.
    pub fn has_verb(&self) -> bool {
        match self {
            Ty::Verb(..) => true,
            Ty::List(t) | Ty::Set(t) | Ty::Maybe(t) => t.has_verb(),
            Ty::Map(k, v) => k.has_verb() || v.has_verb(),
            Ty::Record(r) => r.fields.iter().any(|(_, t)| matches!(t, Ty::Verb(..))),
            _ => false,
        }
    }

    pub fn is_feminine(&self) -> bool {
        match self {
            Ty::List(_) => true,
            Ty::Param(p) => p.feminine,
            Ty::Record(r) => r.feminine,
            Ty::Enum(e) => e.feminine,
            _ => false,
        }
    }

    /// O substantivo do tipo ("inteiro", "texto", "lista de reais").
    pub fn noun(&self, plural: bool) -> String {
        let pick = |s: &str, p: &str| if plural { p.to_string() } else { s.to_string() };
        match self {
            Ty::Int(i) => i.noun(plural),
            Ty::Real(64) => pick("real", "reais"),
            Ty::Real(_) => pick("real de 32 bits", "reais de 32 bits"),
            Ty::Bool => pick("lógico", "lógicos"),
            Ty::Char => pick("caractere", "caracteres"),
            Ty::Text => pick("texto", "textos"),
            Ty::List(t) => format!("{} de {}", if plural { "listas" } else { "lista" }, t.noun(true)),
            Ty::Set(t) => format!("{} de {}", if plural { "conjuntos" } else { "conjunto" }, t.noun(true)),
            Ty::Map(k, v) => format!("{} de {} para {}", if plural { "mapas" } else { "mapa" }, k.noun(true), v.noun(true)),
            Ty::Record(r) => pick(&r.name, &r.plural),
            Ty::Enum(e) => pick(&e.name, &e.plural),
            Ty::Maybe(t) => format!("talvez {}", if plural { t.noun(true) } else { t.describe() }),
            Ty::Verb(params, ret) => {
                let mut s = if plural { "verbos".to_string() } else { "verbo".to_string() };
                if !params.is_empty() {
                    let each: Vec<String> = params
                        .iter()
                        .map(|(t, borrowed)| match (borrowed, t.is_feminine()) {
                            (false, _) => t.noun(false),
                            (true, true) => format!("{} emprestada", t.noun(false)),
                            (true, false) => format!("{} emprestado", t.noun(false)),
                        })
                        .collect();
                    s.push_str(&format!(" de {}", each.join(" e ")));
                }
                if **ret != Ty::Unit {
                    s.push_str(&format!(" para {}", ret.noun(false)));
                }
                s
            }
            Ty::Param(p) if plural => format!("{} quaisquer", crate::morphology::pluralize(&p.name)),
            Ty::Param(p) => format!("{} qualquer", p.name),
            Ty::Unit => pick("nada", "nada"),
            Ty::Infer(_) | Ty::Error => pick("valor", "valores"),
        }
    }

    /// Com artigo indefinido: "um inteiro", "uma lista de textos".
    pub fn describe(&self) -> String {
        if let Ty::Maybe(_) = self {
            return self.noun(false);
        }
        format!("{} {}", if self.is_feminine() { "uma" } else { "um" }, self.noun(false))
    }
}
