//! Cada código de diagnóstico tem explicação (`src/explain/LXXXX.md`) com o título do
//! registro, e os exemplos dela se comportam: o errado dá o código (como primeiro erro;
//! o aviso, como aviso; o de execução, ao rodar no interpretador) e o certo não dá nada.

use lavra::codes;
use lavra::diagnostics::Category;
use lavra::explain::{self, Example};

/// Códigos de construções que o parser ainda não produz: têm texto, sem exemplo.
/// L0010 e L0322 ficaram sem caminho: o léxico só monta real com algarismos dos dois lados (o
/// grande demais é L0007), e a chamada sem valor é acusada antes, pelo L0346.
const WITHOUT_EXAMPLE: &[&str] = &["L0010", "L0304", "L0322", "L0332", "L0353"];

/// Roda o programa no interpretador, numa pilha grande (a recursão da Lavra vai fundo).
/// Devolve o código do erro de execução, se houver.
fn run(program: &str, input: &str) -> Result<(), Option<&'static str>> {
    let (prog, _) = lavra::compile(program).map_err(|_| None)?;
    let input = input.as_bytes().to_vec();
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(1 << 30)
            .spawn_scoped(s, || {
                let mut m = lavra::interp::Machine::new(&prog, std::io::sink(), &input[..]);
                m.run().map_err(|f| f.code.map(|c| c.id))
            })
            .unwrap()
            .join()
            .unwrap()
    })
}

fn check(code: codes::Code, wrong: &Example, right: &Example) -> Vec<String> {
    let mut problems = Vec::new();
    let w = lavra::analyze(&wrong.program);
    let first_error = w.diagnostics.iter().find(|d| d.is_error()).and_then(|d| d.code);
    match code.category {
        Category::Runtime => {
            if w.has_errors() {
                problems.push(format!("o exemplo errado nem compila: {:?}", first_error));
            } else {
                match run(&wrong.program, &wrong.input) {
                    Err(Some(c)) if c == code.id => {}
                    other => problems.push(format!("o exemplo errado, ao rodar, deu {other:?}")),
                }
            }
        }
        Category::Warning => {
            if w.has_errors() {
                problems.push(format!("o exemplo errado tem erro ({first_error:?}), e devia ter só o aviso"));
            }
            if !w.diagnostics.iter().any(|d| d.code == Some(code.id)) {
                problems.push("o exemplo errado não dá o aviso".into());
            }
        }
        _ => {
            if first_error != Some(code.id) {
                problems.push(format!("o primeiro erro do exemplo errado é {first_error:?}"));
            }
        }
    }
    let r = lavra::analyze(&right.program);
    if let Some(d) = r.diagnostics.first() {
        problems.push(format!("o exemplo certo dá {:?}: {}", d.code, d.message));
    } else if code.category == Category::Runtime && run(&right.program, &right.input).is_err() {
        problems.push("o exemplo certo não roda até o fim".into());
    }
    problems
}

#[test]
fn every_code_is_explained_with_examples_that_hold() {
    let mut failures = Vec::new();
    for code in codes::ALL {
        let Some(text) = explain::text(code.id) else {
            failures.push(format!("{}: falta src/explain/{}.md", code.id, code.id));
            continue;
        };
        let title = format!("# {}: {}", code.id, code.title);
        if text.lines().next() != Some(title.as_str()) {
            failures.push(format!("{}: a primeira linha devia ser \"{title}\"", code.id));
        }
        if text.contains(['\u{2014}', '\u{2013}']) {
            failures.push(format!("{}: o texto tem travessão", code.id));
        }
        if WITHOUT_EXAMPLE.contains(&code.id) {
            continue;
        }
        match explain::examples(text) {
            (Some(wrong), Some(right)) => {
                for p in check(*code, &wrong, &right) {
                    failures.push(format!("{}: {p}", code.id));
                }
            }
            _ => failures.push(format!("{}: faltam os exemplos \"Errado\" e \"Certo\"", code.id)),
        }
    }
    for (id, _) in explain::TEXTS {
        if codes::find(id).is_none_or(|c| c.id != *id) {
            failures.push(format!("{id}: explicação de um código que não existe"));
        }
    }
    assert!(failures.is_empty(), "{} problemas nas explicações:\n{}", failures.len(), failures.join("\n"));
}
