//! As tabelas de símbolos que a análise produz para o editor: de um uso se chega à
//! declaração, de uma declaração aos usos, de uma posição ao tipo e aos nomes visíveis.

use lavra::ast::ParamMode;
use lavra::tables::{DefKind, Use};

const PROGRAM: &str = "\
Uma conta é um registro de:
    o titular, um texto;
    o saldo, um inteiro.

Para render a conta, sendo a conta uma conta confiada:
    O saldo da conta cresce 10.

Para começar:
    A carteira começa em uma conta com o titular \"Ana\" e o saldo 100.
    Rendo a carteira.
    Escrevo o saldo da carteira.
";

fn at(needle: &str, nth: usize) -> u32 {
    PROGRAM.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("sem \"{needle}\" #{nth}")).0 as u32
}

#[test]
fn from_a_use_to_the_declaration_and_back() {
    let a = lavra::analyze(PROGRAM);
    assert!(!a.has_errors(), "{:?}", a.diagnostics);
    let t = &a.tables;
    // "a carteira" em "Escrevo o saldo da carteira" leva à declaração.
    let def = t.def_at(at("carteira", 2)).expect("uso de \"a carteira\"");
    let d = &t.defs[def];
    assert_eq!((d.kind, d.name.as_str(), d.display.as_str()), (DefKind::Local, "carteira", "a carteira"));
    assert_eq!(d.span.start, at("A carteira", 0));
    // Os usos: a chamada confiada muda, a escrita lê.
    let uses: Vec<Use> = t.refs_of(def).iter().map(|r| r.use_).collect();
    assert_eq!(uses, [Use::Write, Use::Read]);
    // O verbo da chamada leva à função, e a função conhece o uso.
    let f = t.def_at(at("Rendo", 0)).expect("chamada");
    assert_eq!((t.defs[f].kind, t.defs[f].name.as_str()), (DefKind::Function, "render"));
    assert_eq!(t.refs_of(f).len(), 1);
    // O campo usado leva ao campo do registro.
    let campo = t.def_at(at("saldo da carteira", 0)).expect("campo");
    assert_eq!((t.defs[campo].kind, t.defs[campo].name.as_str()), (DefKind::Field, "saldo"));
    // O parâmetro conhece a função e o modo.
    let p = t.defs.iter().find(|d| d.kind == DefKind::Param).expect("parâmetro");
    assert_eq!((p.name.as_str(), p.mode), ("conta", Some(ParamMode::Lent)));
    assert_eq!(t.args.iter().map(|a| a.1).collect::<Vec<_>>(), [ParamMode::Lent]);
}

#[test]
fn types_and_visible_names_at_a_position() {
    let a = lavra::analyze(PROGRAM);
    let t = &a.tables;
    let (_, ty) = t.type_at(at("saldo da carteira", 0)).expect("tipo do campo");
    assert_eq!(ty.describe(), "um inteiro");
    let here = at("Escrevo", 0);
    let names: Vec<&str> = t.visible_at(here).iter().map(|&i| t.defs[i].name.as_str()).collect();
    assert!(names.contains(&"carteira") && names.contains(&"render") && names.contains(&"conta"), "{names:?}");
    // O parâmetro da outra função não vale aqui.
    assert!(!t.visible_at(here).iter().any(|&i| t.defs[i].kind == DefKind::Param));
}
