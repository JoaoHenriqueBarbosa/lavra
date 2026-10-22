//! Tipos da Lavra depois da análise, e como se dizem em português.

use std::sync::Arc;

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
#[derive(Debug, PartialEq)]
pub struct RecordTy {
    pub id: usize,
    pub name: String,
    pub plural: String,
    pub feminine: bool,
    pub fields: Vec<(String, Ty)>,
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
#[derive(Debug, PartialEq)]
pub struct EnumTy {
    pub id: usize,
    pub name: String,
    pub plural: String,
    pub feminine: bool,
    pub variants: Vec<VariantTy>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int(IntTy),
    Real(u8),
    Bool,
    Char,
    Text,
    List(Box<Ty>),
    Record(Arc<RecordTy>),
    Enum(Arc<EnumTy>),
    /// "talvez um texto": um valor ou nada.
    Maybe(Box<Ty>),
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
        matches!(self, Ty::Int(_) | Ty::Real(_) | Ty::Bool | Ty::Char | Ty::Unit | Ty::Error)
    }

    pub fn is_feminine(&self) -> bool {
        match self {
            Ty::List(_) => true,
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
            Ty::Record(r) => pick(&r.name, &r.plural),
            Ty::Enum(e) => pick(&e.name, &e.plural),
            Ty::Maybe(t) => format!("talvez {}", if plural { t.noun(true) } else { t.describe() }),
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
