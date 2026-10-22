//! As operações embutidas: o que a língua faz por dentro e as bibliotecas declaram em
//! Lavra. Cada operação tem o cabeçalho numa biblioteca (`src/libraries/`), lido pelo
//! próprio parser, e a implementação dupla: no interpretador (`interpret`) e no runtime
//! (`runtime/lib.rs`, a função `symbol`). A regência, os tipos, a posse e o que o editor
//! mostra saem do cabeçalho, então uma operação nova toca só esses quatro lugares.
//!
//! As operações recebem os argumentos emprestados e devolvem um valor novo. No nativo,
//! cada uma recebe também o trecho da chamada (`lo`, `hi`), para relatar uma falha.

use crate::interp::{Fault, Value};
use crate::span::Span;

/// O começo da faixa de posições das bibliotecas. Um programa nunca chega a 1 GiB, então
/// uma posição daqui em diante é de uma biblioteca, e não do arquivo do programa.
pub const LIBRARY_BASE: u32 = 1 << 30;
/// O tamanho da faixa de cada biblioteca.
const LIBRARY_SIZE: u32 = 1 << 24;

/// Uma biblioteca: o nome com que o programa a traz ("Uso a biblioteca de textos.") e os
/// cabeçalhos das operações, em Lavra.
pub struct Library {
    pub name: &'static str,
    pub source: &'static str,
}

pub const LIBRARIES: &[Library] = &[Library { name: "textos", source: include_str!("libraries/textos.lav") }];

/// A biblioteca de nome `name`, com a posição em que as posições dela começam.
pub fn library(name: &str) -> Option<(u32, &'static Library)> {
    LIBRARIES.iter().enumerate().find(|(_, l)| l.name == name).map(|(i, l)| (LIBRARY_BASE + i as u32 * LIBRARY_SIZE, l))
}

/// A biblioteca de uma posição que não é do arquivo do programa.
pub fn library_at(pos: u32) -> Option<&'static Library> {
    let i = pos.checked_sub(LIBRARY_BASE)? / LIBRARY_SIZE;
    LIBRARIES.get(i as usize)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Builtin {
    Split,
    Join,
    Contains,
    Trim,
}

impl Builtin {
    pub const ALL: &[Builtin] = &[Builtin::Split, Builtin::Join, Builtin::Contains, Builtin::Trim];

    /// A biblioteca e o verbo (no infinitivo) que declaram a operação.
    pub fn declared_as(self) -> (&'static str, &'static str) {
        match self {
            Builtin::Split => ("textos", "dividir"),
            Builtin::Join => ("textos", "juntar"),
            Builtin::Contains => ("textos", "conter"),
            Builtin::Trim => ("textos", "aparar"),
        }
    }

    /// A operação que o verbo `infinitive` da biblioteca `library` declara.
    pub fn find(library: &str, infinitive: &str) -> Option<Builtin> {
        Builtin::ALL.iter().find(|b| b.declared_as() == (library, infinitive)).copied()
    }

    pub fn infinitive(self) -> &'static str {
        self.declared_as().1
    }

    /// A função do runtime que faz a operação no nativo.
    pub fn symbol(self) -> &'static str {
        match self {
            Builtin::Split => "rt_text_split",
            Builtin::Join => "rt_text_join",
            Builtin::Contains => "rt_text_contains",
            Builtin::Trim => "rt_text_trim",
        }
    }
}

/// "dividir": os pedaços entre as vezes em que o separador aparece; o separador vazio dá
/// cada caractere.
pub fn split(text: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() { text.chars().map(String::from).collect() } else { text.split(separator).map(String::from).collect() }
}

/// A operação no interpretador, sobre os valores dos argumentos na ordem do cabeçalho.
pub fn interpret(b: Builtin, args: &[Value], _span: Span) -> Result<Value, Fault> {
    let text = |v: &Value| match v {
        Value::Text(t) => t.clone(),
        other => unreachable!("{b:?} recebeu {other:?} onde o cabeçalho pede texto"),
    };
    Ok(match b {
        Builtin::Split => Value::List(split(&text(&args[0]), &text(&args[1])).into_iter().map(Value::Text).collect()),
        Builtin::Join => {
            let Value::List(items) = &args[0] else { unreachable!("\"juntar\" recebe uma lista") };
            Value::Text(items.iter().map(text).collect::<Vec<_>>().join(&text(&args[1])))
        }
        Builtin::Contains => Value::Bool(text(&args[0]).contains(&text(&args[1]))),
        Builtin::Trim => Value::Text(text(&args[0]).trim().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParamMode;
    use std::collections::HashSet;

    #[test]
    fn every_library_declares_its_operations_and_only_them() {
        for lib in LIBRARIES {
            let src = format!("Uso a biblioteca de {}.\n\nPara começar:\n    Escrevo 1.\n", lib.name);
            let a = crate::analyze(&src);
            assert!(a.diagnostics.is_empty(), "a biblioteca de {} dá {:?}", lib.name, a.diagnostics);
            let sigs: Vec<_> = a.ast.sigs.iter().filter(|s| s.builtin.is_some()).collect();
            let declared: HashSet<Builtin> = sigs.iter().filter_map(|s| s.builtin).collect();
            let expected: HashSet<Builtin> = Builtin::ALL.iter().filter(|b| b.declared_as().0 == lib.name).copied().collect();
            assert_eq!(declared, expected, "as operações da biblioteca de {}", lib.name);
            assert_eq!(sigs.len(), expected.len(), "cada operação de {} uma vez só", lib.name);
            for s in sigs {
                assert!(!s.fallible, "\"{}\" não falha pela regência", s.infinitive);
                for p in &s.params {
                    assert_eq!(p.mode, ParamMode::Borrowed, "\"{}\" recebe {} emprestado", s.infinitive, p.name.display());
                }
            }
        }
        for b in Builtin::ALL {
            assert!(library(b.declared_as().0).is_some(), "{b:?} está numa biblioteca que existe");
        }
    }

    #[test]
    fn library_positions_stay_out_of_the_program() {
        let (base, lib) = library("textos").unwrap();
        assert!(base >= LIBRARY_BASE);
        assert_eq!(library_at(base + 10).map(|l| l.name), Some(lib.name));
        assert!(library_at(100).is_none());
    }
}
