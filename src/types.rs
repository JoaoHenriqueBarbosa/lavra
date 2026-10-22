//! Tipos da Lavra depois da análise, e como se dizem em português.

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

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int(IntTy),
    Real(u8),
    Bool,
    Char,
    Text,
    Unit,
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

    /// O substantivo do tipo ("inteiro", "texto").
    pub fn noun(&self, plural: bool) -> String {
        let pick = |s: &str, p: &str| if plural { p.to_string() } else { s.to_string() };
        match self {
            Ty::Int(i) => i.noun(plural),
            Ty::Real(64) => pick("real", "reais"),
            Ty::Real(_) => pick("real de 32 bits", "reais de 32 bits"),
            Ty::Bool => pick("lógico", "lógicos"),
            Ty::Char => pick("caractere", "caracteres"),
            Ty::Text => pick("texto", "textos"),
            Ty::Unit => pick("nada", "nada"),
            Ty::Error => pick("valor", "valores"),
        }
    }

    /// Com artigo indefinido: "um inteiro", "um texto".
    pub fn describe(&self) -> String {
        format!("um {}", self.noun(false))
    }
}
