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
use crate::textfmt;

/// O começo da faixa de posições das bibliotecas. Um programa nunca chega a 1 GiB, então
/// uma posição daqui em diante é de uma biblioteca, e não do arquivo do programa.
pub const LIBRARY_BASE: u32 = 1 << 30;
/// O tamanho da faixa de cada biblioteca.
const LIBRARY_SIZE: u32 = 1 << 24;

/// Uma biblioteca: o nome com que o programa a traz ("Uso a biblioteca de textos.") e os
/// cabeçalhos das operações, em Lavra.
pub struct Library {
    pub name: &'static str,
    /// O nome do arquivo, sem acento: é o que vai no endereço que o editor abre.
    pub file: &'static str,
    pub source: &'static str,
}

pub const LIBRARIES: &[Library] = &[
    Library { name: "textos", file: "textos", source: include_str!("libraries/textos.lav") },
    Library { name: "matemática", file: "matematica", source: include_str!("libraries/matematica.lav") },
    Library { name: "arquivos", file: "arquivos", source: include_str!("libraries/arquivos.lav") },
    Library { name: "tempo", file: "tempo", source: include_str!("libraries/tempo.lav") },
];

/// O estado dos sorteios do interpretador; sem semente, começa pelo relógio.
static RANDOM: std::sync::Mutex<Option<u64>> = std::sync::Mutex::new(None);

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
    Upper,
    Lower,
    Reverse,
    Replace,
    Locate,
    Slice,
    Sin,
    Cos,
    Tan,
    Ln,
    Exp,
    Floor,
    Ceil,
    Pi,
    Round,
    Random,
    Seed,
    FileLoad,
    FileSave,
    FileAppend,
    FileRemove,
    FileExists,
    DirList,
    Now,
    /// Uma parte do instante: 0 é o ano, e daí mês, dia, hora e minuto.
    TimePart(u8),
    DateText,
    Sleep,
}

impl Builtin {
    pub const ALL: &[Builtin] = &[
        Builtin::Split,
        Builtin::Join,
        Builtin::Contains,
        Builtin::Trim,
        Builtin::Upper,
        Builtin::Lower,
        Builtin::Reverse,
        Builtin::Replace,
        Builtin::Locate,
        Builtin::Slice,
        Builtin::Sin,
        Builtin::Cos,
        Builtin::Tan,
        Builtin::Ln,
        Builtin::Exp,
        Builtin::Floor,
        Builtin::Ceil,
        Builtin::Pi,
        Builtin::Round,
        Builtin::Random,
        Builtin::Seed,
        Builtin::FileLoad,
        Builtin::FileSave,
        Builtin::FileAppend,
        Builtin::FileRemove,
        Builtin::FileExists,
        Builtin::DirList,
        Builtin::Now,
        Builtin::TimePart(0),
        Builtin::TimePart(1),
        Builtin::TimePart(2),
        Builtin::TimePart(3),
        Builtin::TimePart(4),
        Builtin::DateText,
        Builtin::Sleep,
    ];

    /// A operação pode falhar ("salvo engano"): quem chama trata, e o nativo recebe onde
    /// deixar a falha.
    pub fn fallible(self) -> bool {
        matches!(self, Builtin::FileLoad | Builtin::FileSave | Builtin::FileAppend | Builtin::FileRemove | Builtin::DirList)
    }

    /// A biblioteca e o verbo (no infinitivo) que declaram a operação. A operação por
    /// extenso leva o nome junto: "calcular o seno".
    pub fn declared_as(self) -> (&'static str, &'static str) {
        match self {
            Builtin::Split => ("textos", "dividir"),
            Builtin::Join => ("textos", "juntar"),
            Builtin::Contains => ("textos", "conter"),
            Builtin::Trim => ("textos", "aparar"),
            Builtin::Upper => ("textos", "maiusculizar"),
            Builtin::Lower => ("textos", "minusculizar"),
            Builtin::Reverse => ("textos", "inverter"),
            Builtin::Replace => ("textos", "substituir"),
            Builtin::Locate => ("textos", "localizar"),
            Builtin::Slice => ("textos", "recortar"),
            Builtin::Sin => ("matemática", "calcular o seno"),
            Builtin::Cos => ("matemática", "calcular o cosseno"),
            Builtin::Tan => ("matemática", "calcular a tangente"),
            Builtin::Ln => ("matemática", "calcular o logaritmo"),
            Builtin::Exp => ("matemática", "calcular a exponencial"),
            Builtin::Floor => ("matemática", "calcular o piso"),
            Builtin::Ceil => ("matemática", "calcular o teto"),
            Builtin::Pi => ("matemática", "calcular o pi"),
            Builtin::Round => ("matemática", "arredondar"),
            Builtin::Random => ("matemática", "sortear"),
            Builtin::Seed => ("matemática", "semear"),
            Builtin::FileLoad => ("arquivos", "carregar"),
            Builtin::FileSave => ("arquivos", "gravar"),
            Builtin::FileAppend => ("arquivos", "acrescentar"),
            Builtin::FileRemove => ("arquivos", "apagar"),
            Builtin::FileExists => ("arquivos", "existir"),
            Builtin::DirList => ("arquivos", "listar"),
            Builtin::Now => ("tempo", "calcular o agora"),
            Builtin::TimePart(0) => ("tempo", "calcular o ano"),
            Builtin::TimePart(1) => ("tempo", "calcular o mês"),
            Builtin::TimePart(2) => ("tempo", "calcular o dia"),
            Builtin::TimePart(3) => ("tempo", "calcular a hora"),
            Builtin::TimePart(_) => ("tempo", "calcular o minuto"),
            Builtin::DateText => ("tempo", "datar"),
            Builtin::Sleep => ("tempo", "aguardar"),
        }
    }

    /// A operação que o verbo `infinitive` da biblioteca `library` declara.
    pub fn find(library: &str, infinitive: &str) -> Option<Builtin> {
        Builtin::ALL.iter().find(|b| b.declared_as() == (library, infinitive)).copied()
    }

    /// O verbo da operação; da operação por extenso, o nome ("seno").
    pub fn infinitive(self) -> &'static str {
        let key = self.declared_as().1;
        key.rsplit(' ').next().unwrap_or(key)
    }

    /// A operação por extenso ("o seno de x") de nome `word`, de alguma biblioteca.
    pub fn nominal(word: &str) -> Option<Builtin> {
        Builtin::ALL.iter().copied().find(|b| b.declared_as().1.contains(' ') && b.infinitive() == word)
    }

    /// É uma operação por extenso: usa-se pelo nome, e não por um verbo.
    pub fn is_nominal(self) -> bool {
        self.declared_as().1.contains(' ')
    }

    /// A função do runtime que faz a operação no nativo.
    pub fn symbol(self) -> &'static str {
        match self {
            Builtin::Split => "rt_text_split",
            Builtin::Join => "rt_text_join",
            Builtin::Contains => "rt_text_contains",
            Builtin::Trim => "rt_text_trim",
            Builtin::Upper => "rt_text_upper",
            Builtin::Lower => "rt_text_lower",
            Builtin::Reverse => "rt_text_reverse",
            Builtin::Replace => "rt_text_replace",
            Builtin::Locate => "rt_text_locate",
            Builtin::Slice => "rt_text_slice",
            Builtin::Sin => "rt_math_sin",
            Builtin::Cos => "rt_math_cos",
            Builtin::Tan => "rt_math_tan",
            Builtin::Ln => "rt_math_ln",
            Builtin::Exp => "rt_math_exp",
            Builtin::Floor => "rt_math_floor",
            Builtin::Ceil => "rt_math_ceil",
            Builtin::Pi => "rt_math_pi",
            Builtin::Round => "rt_math_round",
            Builtin::Random => "rt_math_random",
            Builtin::Seed => "rt_math_seed",
            Builtin::FileLoad => "rt_file_load",
            Builtin::FileSave => "rt_file_save",
            Builtin::FileAppend => "rt_file_append",
            Builtin::FileRemove => "rt_file_remove",
            Builtin::FileExists => "rt_file_exists",
            Builtin::DirList => "rt_dir_list",
            Builtin::Now => "rt_time_now",
            Builtin::TimePart(0) => "rt_time_year",
            Builtin::TimePart(1) => "rt_time_month",
            Builtin::TimePart(2) => "rt_time_day",
            Builtin::TimePart(3) => "rt_time_hour",
            Builtin::TimePart(_) => "rt_time_minute",
            Builtin::DateText => "rt_time_text",
            Builtin::Sleep => "rt_time_sleep",
        }
    }
}

/// "dividir": os pedaços entre as vezes em que o separador aparece; o separador vazio dá
/// cada caractere.
pub fn split(text: &str, separator: &str) -> Vec<String> {
    if separator.is_empty() { text.chars().map(String::from).collect() } else { text.split(separator).map(String::from).collect() }
}

/// A operação no interpretador, sobre os valores dos argumentos na ordem do cabeçalho.
pub fn interpret(b: Builtin, args: &[Value], span: Span) -> Result<Value, Fault> {
    let int = |v: &Value| match v {
        Value::Int(n) => *n,
        other => unreachable!("{b:?} recebeu {other:?} onde o cabeçalho pede inteiro"),
    };
    let text = |v: &Value| match v {
        Value::Text(t) => t.clone(),
        other => unreachable!("{b:?} recebeu {other:?} onde o cabeçalho pede texto"),
    };
    let real = |v: &Value| match v {
        Value::Real(x) => *x,
        other => unreachable!("{b:?} recebeu {other:?} onde o cabeçalho pede real"),
    };
    // O real já arredondado vira inteiro, se couber.
    let whole = |x: f64| {
        if textfmt::real_fits(x, 64, true) {
            Ok(Value::Int(x as i128))
        } else {
            Err(Fault {
                code: Some(crate::codes::L0908),
                message: textfmt::convert_error(&textfmt::format_real(x), 64, true),
                span,
                desist: false,
            })
        }
    };
    // A falha declarada da operação, que sobe até quem a trata.
    let failed = |message: String| Fault { code: None, message, span, desist: true };
    Ok(match b {
        Builtin::Now => Value::Int(textfmt::now_millis() as i128),
        Builtin::TimePart(part) => Value::Int(textfmt::civil(int(&args[0]) as i64)[part as usize % 6] as i128),
        Builtin::DateText => Value::Text(textfmt::date_text(int(&args[0]) as i64)),
        Builtin::Sleep => {
            textfmt::sleep_millis(int(&args[0]) as i64);
            Value::Unit
        }
        Builtin::FileLoad => Value::Text(textfmt::file_load(&text(&args[0])).map_err(failed)?),
        Builtin::FileSave => textfmt::file_save(&text(&args[0]), &text(&args[1]), false).map(|_| Value::Unit).map_err(failed)?,
        Builtin::FileAppend => textfmt::file_save(&text(&args[0]), &text(&args[1]), true).map(|_| Value::Unit).map_err(failed)?,
        Builtin::FileRemove => textfmt::file_remove(&text(&args[0])).map(|_| Value::Unit).map_err(failed)?,
        Builtin::FileExists => Value::Bool(textfmt::file_exists(&text(&args[0]))),
        Builtin::DirList => Value::List(textfmt::dir_list(&text(&args[0])).map_err(failed)?.into_iter().map(Value::Text).collect()),
        Builtin::Sin => Value::Real(real(&args[0]).sin()),
        Builtin::Cos => Value::Real(real(&args[0]).cos()),
        Builtin::Tan => Value::Real(real(&args[0]).tan()),
        Builtin::Ln => {
            let x = real(&args[0]);
            if x.is_nan() || x <= 0.0 {
                return Err(Fault { code: Some(crate::codes::L0918), message: textfmt::log_domain(x), span, desist: false });
            }
            Value::Real(x.ln())
        }
        Builtin::Exp => Value::Real(real(&args[0]).exp()),
        Builtin::Floor => whole(real(&args[0]).floor())?,
        Builtin::Ceil => whole(real(&args[0]).ceil())?,
        Builtin::Round => whole(real(&args[0]).round())?,
        Builtin::Pi => Value::Real(std::f64::consts::PI),
        Builtin::Random => {
            let mut state = RANDOM.lock().unwrap_or_else(|e| e.into_inner());
            let s = state.get_or_insert_with(textfmt::clock_seed);
            Value::Int(textfmt::random_between(s, int(&args[0]) as i64, int(&args[1]) as i64) as i128)
        }
        Builtin::Seed => {
            *RANDOM.lock().unwrap_or_else(|e| e.into_inner()) = Some(int(&args[0]) as u64);
            Value::Unit
        }
        Builtin::Split => Value::List(split(&text(&args[0]), &text(&args[1])).into_iter().map(Value::Text).collect()),
        Builtin::Join => {
            let Value::List(items) = &args[0] else { unreachable!("\"juntar\" recebe uma lista") };
            Value::Text(items.iter().map(text).collect::<Vec<_>>().join(&text(&args[1])))
        }
        Builtin::Contains => Value::Bool(text(&args[0]).contains(&text(&args[1]))),
        Builtin::Trim => Value::Text(text(&args[0]).trim().to_string()),
        Builtin::Upper => Value::Text(text(&args[0]).to_uppercase()),
        Builtin::Lower => Value::Text(text(&args[0]).to_lowercase()),
        Builtin::Reverse => Value::Text(text(&args[0]).chars().rev().collect()),
        Builtin::Replace => Value::Text(textfmt::replace(&text(&args[2]), &text(&args[0]), &text(&args[1]))),
        Builtin::Locate => match textfmt::locate(&text(&args[1]), &text(&args[0])) {
            Some(p) => Value::Int(p as i128),
            None => Value::Nothing,
        },
        Builtin::Slice => match textfmt::slice(&text(&args[0]), int(&args[1]), int(&args[2])) {
            Ok(t) => Value::Text(t),
            Err(message) => return Err(Fault { code: Some(crate::codes::L0917), message, span, desist: false }),
        },
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
                assert_eq!(s.fallible, s.builtin.unwrap().fallible(), "\"{}\" e o \"salvo engano\" do cabeçalho", s.infinitive);
                for p in &s.params {
                    // O que tem dono vem emprestado; número, lógico e caractere se copiam.
                    let copies = matches!(
                        p.ty.ty,
                        crate::ast::TypeExpr::Int { .. }
                            | crate::ast::TypeExpr::Byte
                            | crate::ast::TypeExpr::Real { .. }
                            | crate::ast::TypeExpr::Bool
                            | crate::ast::TypeExpr::Char
                    );
                    let expected = if copies { ParamMode::Owned } else { ParamMode::Borrowed };
                    assert_eq!(p.mode, expected, "\"{}\" recebe {} emprestado", s.infinitive, p.name.display());
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
