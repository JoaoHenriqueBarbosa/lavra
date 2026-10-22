//! O rename: cada forma de cada nome, a concordância que vai junto, as recusas, e o
//! corpus inteiro renomeado nome a nome, compilando e com a mesma saída.

use lavra::diagnostics::Edit;
use lavra::rename::{prepare, rename};
use lavra::tables::DefKind;
use std::path::{Path, PathBuf};

fn apply(src: &str, edits: &[Edit]) -> String {
    let mut out = src.to_string();
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.span.start));
    for e in sorted {
        out.replace_range(e.span.start as usize..e.span.end as usize, &e.text);
    }
    out
}

/// Renomeia o nome escrito em `at` (a primeira vez que o trecho aparece).
fn renamed(src: &str, at: &str, new: &str) -> Result<String, String> {
    let offset = src.find(at).unwrap_or_else(|| panic!("{at:?} não está no programa")) as u32;
    rename(src, offset, new).map(|e| apply(src, &e))
}

fn output(src: &str, stdin: &[u8]) -> String {
    let (prog, _) = lavra::compile(src).unwrap_or_else(|d| panic!("não compila: {}\n{src}", d[0].message));
    let mut out = Vec::new();
    let _ = lavra::interp::Machine::new(&prog, &mut out, stdin).run();
    String::from_utf8(out).unwrap()
}

const FUNCOES: &str = include_str!("programs/ok/funcoes.lav");

#[test]
fn verbs_in_every_form() {
    // Ação: o cabeçalho, a 1ª pessoa (maiúscula no começo da oração) e o infinitivo.
    let r = renamed(FUNCOES, "elevar x", "potenciar").unwrap();
    assert!(r.contains("Para potenciar x a n,"), "{r}");
    assert!(r.contains("seguido do resultado de potenciar 2 a 10."), "{r}");
    assert!(r.contains("    Potencio 1,5 a 2 e escrevo o resultado."), "{r}");
    assert_eq!(output(&r, b""), output(FUNCOES, b""));
    // Predicado: o futuro do subjuntivo nas condições.
    let r = renamed(FUNCOES, "exceder o limite", "superar").unwrap();
    assert!(r.contains("Para o valor superar o limite,"), "{r}");
    assert!(r.contains("Se 120 superar o teto:") && r.contains("Se o teto não superar 120:"), "{r}");
    // Verbo irregular: "caber" dá "couber" no subjuntivo.
    let r = renamed(FUNCOES, "exceder o limite", "caber").unwrap();
    assert!(r.contains("Se 120 couber o teto:"), "{r}");
    // Predicado adjetivo: o adjetivo concorda com o sujeito de cada uso.
    let r = renamed(FUNCOES, "par, sendo", "ímpar").unwrap();
    assert!(r.contains("Para o número ser ímpar,") && r.contains("Se i for ímpar:"), "{r}");
    // A 1ª pessoa minúscula no meio da oração, e o complemento fixo que fica.
    let r = renamed(FUNCOES, "render o saldo", "valorizar").unwrap();
    assert!(r.contains("Repito 2 vezes: valorizo o saldo."), "{r}");
    let r = renamed(FUNCOES, "contar os dias", "calcular").unwrap();
    assert!(r.contains("Para calcular os dias até a data,") && r.contains("o resultado de calcular os dias até 2."), "{r}");
    assert_eq!(output(&r, b""), output(FUNCOES, b""));
}

#[test]
fn personal_infinitive_after_the_fact() {
    let src = "\
Para o valor exceder o limite, sendo ambos inteiros, devolvendo um lógico:
    Devolvo o fato de o valor ser maior que o limite.

Para começar:
    O saldo é 10.
    Escrevo o fato de o saldo exceder 5.
    Se o saldo exceder 5: escrevo \"sim\".
";
    // "fazer": o infinitivo pessoal é "fazer" e o subjuntivo é "fizer".
    let r = renamed(src, "exceder o limite", "fazer").unwrap();
    assert!(r.contains("Escrevo o fato de o saldo fazer 5.") && r.contains("Se o saldo fizer 5:"), "{r}");
    assert_eq!(output(&r, b""), output(src, b""));
}

#[test]
fn declared_first_person() {
    let src = "\
Para trovejar (trovejo) a nuvem, sendo a nuvem um texto:
    Escrevo a nuvem.

Para começar:
    Trovejo \"cúmulo\".
";
    let r = renamed(src, "trovejar", "chover (chovo)").unwrap();
    assert!(r.contains("Para chover (chovo) a nuvem,") && r.contains("    Chovo \"cúmulo\"."), "{r}");
    // O verbo novo tem 1ª pessoa regular: os parênteses saem.
    let r = renamed(src, "trovejar", "anunciar").unwrap();
    assert!(r.contains("Para anunciar a nuvem,") && r.contains("    Anuncio \"cúmulo\"."), "{r}");
    // Defectivo sem a forma declarada não executa.
    let e = renamed(&r, "anunciar", "chover").unwrap_err();
    assert!(e.contains("1ª pessoa"), "{e}");
}

const NOMES: &str = "\
O teto é 500.

Para render o saldo, sendo o saldo um inteiro confiado:
    O saldo cresce 10.

Para começar:
    O saldo começa em 100.
    Rendo o saldo.
    Escrevo o negativo do saldo.
    As notas são 7, 8 e 9.
    Escrevo a primeira das notas.
    Escrevo a última das notas.
    Para cada nota:
        Escrevo a nota.
    As compras começam vazias.
    As compras ganham 3.
    Escrevo o tamanho das compras.
";

#[test]
fn names_with_gender_and_contractions() {
    // Mesmo gênero: só a palavra.
    let r = renamed(NOMES, "O saldo começa", "o montante").unwrap();
    assert!(r.contains("    O montante começa em 100.\n    Rendo o montante.\n    Escrevo o negativo do montante."), "{r}");
    // O parâmetro com o mesmo nome é outro nome: fica.
    assert!(r.contains("Para render o saldo, sendo o saldo um inteiro confiado:"), "{r}");
    // Outro gênero: o artigo e a contração vão junto, com a maiúscula.
    let r = renamed(NOMES, "O saldo começa", "a reserva").unwrap();
    assert!(r.contains("    A reserva começa em 100.\n    Rendo a reserva.\n    Escrevo o negativo da reserva."), "{r}");
    assert_eq!(output(&r, b""), output(NOMES, b""));
    // O parâmetro: a regência, a tipagem e o corpo.
    let r = renamed(NOMES, "o saldo, sendo", "a quantia").unwrap();
    assert!(r.contains("Para render a quantia, sendo a quantia um inteiro confiado:\n    A quantia cresce 10."), "{r}");
    // Sem artigo, o gênero sai da terminação.
    let r = renamed(NOMES, "O saldo começa", "reserva").unwrap();
    assert!(r.contains("A reserva começa em 100."), "{r}");
}

#[test]
fn collections_carry_ordinals_items_and_adjectives() {
    // A coleção muda de gênero: o ordinal vai junto, e o item de "Para cada nota" vai
    // para o singular do nome novo.
    let r = renamed(NOMES, "As notas", "os valores").unwrap();
    assert!(r.contains("    Os valores são 7, 8 e 9.\n    Escrevo o primeiro dos valores.\n    Escrevo o último dos valores."), "{r}");
    assert!(r.contains("    Para cada valor:\n        Escrevo o valor."), "{r}");
    assert_eq!(output(&r, b""), output(NOMES, b""));
    // O "vazias" concorda com o nome sem citá-lo: a correção do compilador acerta.
    let r = renamed(NOMES, "As compras", "os pedidos").unwrap();
    assert!(r.contains("    Os pedidos começam vazios.\n    Os pedidos ganham 3.\n    Escrevo o tamanho dos pedidos."), "{r}");
    // O item escrito que é o singular da coleção vai junto com ela.
    let src = "Para começar:\n    Os herdeiros são 1, 2 e 3.\n    Para cada herdeiro dos herdeiros:\n        Escrevo o herdeiro.\n";
    let r = renamed(src, "Os herdeiros", "as partes").unwrap();
    assert!(r.contains("Para cada parte das partes:\n        Escrevo a parte."), "{r}");
    // O item tem o gênero da coleção: trocar só o dele é recusado, com a regra.
    let e = renamed(src, "o herdeiro.", "a parte").unwrap_err();
    assert!(e.contains("gênero da coleção, \"os herdeiros\" (masculino)"), "{e}");
    // Renomear só o item deixa a coleção explícita.
    let r = renamed(NOMES, "a nota.", "a avaliação").unwrap();
    assert!(r.contains("    Para cada avaliação das notas:\n        Escrevo a avaliação."), "{r}");
    assert_eq!(output(&r, b""), output(NOMES, b""));
}

const TIPOS: &str = "\
Uma conta é um registro de:
    o titular, um texto;
    o saldo, um inteiro.

Uma forma é uma destas:
    um círculo, com o raio, um inteiro;
    um ponto.

Para mostrar a conta, sendo a conta uma conta emprestada:
    Escrevo o titular da conta seguido de \": \" seguido do saldo da conta.

Para começar:
    A carteira começa em uma conta com o titular \"Ana\", o saldo 100.
    O saldo da carteira cresce 50.
    Mostro a carteira.
    As contas, uma lista de contas, começam vazias.
    A figura é um círculo com o raio 2.
    Conforme a figura:
        Sendo um círculo: escrevo o raio.
        Sendo um ponto: escrevo \"ponto\".
    Se a figura for um círculo: escrevo \"redonda\".
";

#[test]
fn types_cases_and_fields() {
    // O tipo muda de gênero: os artigos indefinidos, o particípio e o plural.
    let r = renamed(TIPOS, "conta é um registro", "um cofre").unwrap();
    assert!(r.starts_with("Um cofre é um registro de:"), "{r}");
    assert!(r.contains("sendo a conta um cofre emprestado:"), "{r}");
    assert!(r.contains("em um cofre com o titular"), "{r}");
    assert!(r.contains("As contas, uma lista de cofres, começam vazias."), "{r}");
    assert_eq!(output(&r, b""), output(TIPOS, b""));
    // A enumeração: "uma destas" vira "um destes".
    let r = renamed(TIPOS, "forma é uma", "um formato").unwrap();
    assert!(r.contains("Um formato é um destes:"), "{r}");
    // O caso, com o artigo de cada uso.
    let r = renamed(TIPOS, "círculo, com", "uma elipse").unwrap();
    assert!(r.contains("    uma elipse, com o raio, um inteiro;"), "{r}");
    assert!(r.contains("A figura é uma elipse com o raio 2.") && r.contains("Sendo uma elipse:") && r.contains("for uma elipse:"), "{r}");
    // O campo do caso: a definição, a construção e o nome que o braço expõe.
    let r = renamed(TIPOS, "raio, um", "a medida").unwrap();
    assert!(r.contains("um círculo, com a medida, um inteiro;") && r.contains("com a medida 2.") && r.contains("escrevo a medida."), "{r}");
    assert_eq!(output(&r, b""), output(TIPOS, b""));
    // O campo do registro, de dentro do braço do "Conforme" também.
    let r = renamed(TIPOS, "saldo, um", "a reserva").unwrap();
    assert!(r.contains("    a reserva, um inteiro.") && r.contains("seguido da reserva da conta."), "{r}");
    assert!(r.contains("o titular \"Ana\", a reserva 100.") && r.contains("    A reserva da carteira cresce 50."), "{r}");
    assert_eq!(output(&r, b""), output(TIPOS, b""));
}

#[test]
fn maybe_names_go_together() {
    let src = "\
Para começar:
    O apelido, talvez um texto, começa em nada.
    O apelido vira \"Zé\".
    Se houver apelido: escrevo o apelido.
";
    let r = renamed(src, "apelido, talvez", "a alcunha").unwrap();
    assert!(r.contains("A alcunha, talvez um texto, começa em nada.") && r.contains("Se houver alcunha: escrevo a alcunha."), "{r}");
    // De dentro do "Se houver" também.
    let r = renamed(src, "o apelido.", "a alcunha").unwrap();
    assert!(r.contains("A alcunha vira") && r.contains("Se houver alcunha: escrevo a alcunha."), "{r}");
    assert_eq!(output(&r, b""), output(src, b""));
}

#[test]
fn both_follows_the_parameters() {
    let src = "\
Para o valor exceder o limite, sendo ambos inteiros, devolvendo um lógico:
    Devolvo o fato de o valor ser maior que o limite.

Para começar:
    Se 3 exceder 2: escrevo \"sim\".
";
    let r = renamed(src, "valor exceder", "a quantia").unwrap();
    assert!(r.contains("Para a quantia exceder o limite, sendo ambos inteiros,"), "{r}");
    let r = renamed(&r, "limite, sendo", "a cota").unwrap();
    assert!(r.starts_with("Para a quantia exceder a cota, sendo ambas inteiros, devolvendo um lógico:"), "{r}");
    assert_eq!(output(&r, b""), "sim\n");
}

#[test]
fn refusals_say_why() {
    // O número fica.
    let e = renamed(NOMES, "As notas", "a nota").unwrap_err();
    assert!(e.contains("plural"), "{e}");
    // Sem artigo e sem terminação que decida o gênero.
    let e = renamed(NOMES, "O saldo começa", "valor").unwrap_err();
    assert!(e.contains("\"o valor\" ou \"a valor\""), "{e}");
    // Símbolo troca por símbolo.
    let e = renamed(FUNCOES, "x a n", "a base").unwrap_err();
    assert!(e.contains("símbolo"), "{e}");
    // O nome novo já existe ali: o compilador acusa, e o rename não acontece.
    let e = renamed(NOMES, "O saldo começa", "as notas").unwrap_err();
    assert!(e.contains("singular"), "{e}");
    let e = renamed(NOMES, "As compras", "as notas").unwrap_err();
    assert!(e.contains("erro"), "{e}");
    // O nome novo esconderia outro que o programa usa depois.
    let src = "\
O teto é 500.

Para começar:
    O saldo começa em 100.
    Escrevo o saldo.
    Escrevo o teto.
";
    let e = renamed(src, "O saldo", "o teto").unwrap_err();
    assert!(e.contains("sentido") || e.contains("erro"), "{e}");
    // Palavra da língua não é nome.
    assert!(renamed(NOMES, "O saldo começa", "o resultado").is_err());
    // "o resultado" é da língua.
    let src = "Para começar:\n    Escrevo 2 vezes 3.\n    Escrevo o resultado.\n";
    assert!(rename(src, src.find("resultado").unwrap() as u32, "o produto").is_err());
}

#[test]
fn prepare_shows_the_name_as_written() {
    let t = prepare(NOMES, NOMES.find("do saldo").unwrap() as u32 + 4).unwrap();
    assert_eq!(t.placeholder, "o saldo");
    assert_eq!(&NOMES[t.span.start as usize..t.span.end as usize], "saldo");
    let t = prepare(FUNCOES, FUNCOES.find("Elevo").unwrap() as u32 + 1).unwrap();
    assert_eq!((t.placeholder.as_str(), &FUNCOES[t.span.start as usize..t.span.end as usize]), ("elevar", "Elevo"));
    let t = prepare(TIPOS, TIPOS.find("conta emprestada").unwrap() as u32).unwrap();
    assert_eq!(t.placeholder, "uma conta");
    assert!(prepare(NOMES, NOMES.find("Escrevo").unwrap() as u32).is_err());
}

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["samples", "tests/programs/ok"] {
        for e in std::fs::read_dir(root.join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "lav") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

/// Um nome novo que não está no programa, do outro gênero.
fn fresh(kind: DefKind, display: &str, name: &str, src: &str, plural: bool, feminine: bool) -> Option<String> {
    let pick = |options: &[&str]| options.iter().find(|w| !src.contains(*w)).map(|w| w.to_string());
    match kind {
        DefKind::Function => pick(&["zabumbar", "xeretar", "zunzunar"]),
        _ if lavra::morphology::is_symbol(name) && !display.contains(' ') => pick(&["z7", "z8", "z9"]),
        DefKind::Record | DefKind::Enum | DefKind::Variant => {
            if feminine {
                pick(&["zimbro", "zarolho"]).map(|w| format!("um {w}"))
            } else {
                pick(&["zabumba", "xerereca"]).map(|w| format!("uma {w}"))
            }
        }
        _ => match (plural, feminine) {
            (false, true) => pick(&["zimbro", "zarolho"]).map(|w| format!("o {w}")),
            (false, false) => pick(&["zabumba", "xerereca"]).map(|w| format!("a {w}")),
            (true, true) => pick(&["zimbros", "zarolhos"]).map(|w| format!("os {w}")),
            (true, false) => pick(&["zabumbas", "xererecas"]).map(|w| format!("as {w}")),
        },
    }
}

#[test]
fn every_name_of_the_corpus_renames_and_keeps_the_output() {
    let mut failures = Vec::new();
    let mut renamed_count = 0;
    for path in corpus() {
        let src = std::fs::read_to_string(&path).unwrap();
        let stdin = std::fs::read(path.with_extension("stdin")).unwrap_or_default();
        let a = lavra::analyze(&src);
        if a.has_errors() {
            continue;
        }
        let expected = output(&src, &stdin);
        for (i, d) in a.tables.defs.iter().enumerate() {
            // "o resultado" é da língua, e o que vem de uma biblioteca não se renomeia.
            if d.kind == DefKind::Result || d.span.is_empty() || d.span.start >= lavra::builtins::LIBRARY_BASE {
                continue;
            }
            // A palavra da declaração (o fim do trecho), e o nome que está ali.
            let offset = d.span.end - 1;
            if a.tables.def_at(offset) != Some(i) {
                continue;
            }
            let adjective = d.kind == DefKind::Function && !lavra::morphology::verbs::is_infinitive(&d.name);
            let new = if adjective {
                Some(if d.name.ends_with('a') { "zarolha".to_string() } else { "zarolho".to_string() })
            } else {
                fresh(
                    d.kind,
                    &d.display,
                    &d.name,
                    &src,
                    d.number == lavra::morphology::Number::Plural,
                    d.gender == lavra::morphology::Gender::Feminine,
                )
            };
            let Some(mut new) = new else { continue };
            let mut result = rename(&src, offset, &new);
            // O item de "Para cada" tem o gênero da coleção: troca o nome no mesmo gênero.
            if result.as_ref().is_err_and(|e| e.contains("gênero da coleção")) {
                let same = d.gender != lavra::morphology::Gender::Feminine;
                new = fresh(d.kind, &d.display, &d.name, &src, d.number == lavra::morphology::Number::Plural, same).unwrap();
                result = rename(&src, offset, &new);
            }
            let name = format!("{} ({} → {new})", path.display(), d.display);
            match result {
                Ok(edits) if edits.is_empty() => failures.push(format!("{name}: nada mudou")),
                Ok(edits) => {
                    renamed_count += 1;
                    let r = apply(&src, &edits);
                    let after = lavra::analyze(&r);
                    if after.has_errors() {
                        failures.push(format!("{name}: não compila: {}", after.diagnostics[0].message));
                        continue;
                    }
                    // Tipos, casos e campos aparecem na escrita dos valores; os outros não.
                    let shows = matches!(d.kind, DefKind::Record | DefKind::Enum | DefKind::Variant | DefKind::Field);
                    if !shows && output(&r, &stdin) != expected {
                        failures.push(format!("{name}: a saída mudou"));
                    }
                }
                Err(e) => failures.push(format!("{name}: recusado: {e}")),
            }
        }
    }
    assert!(failures.is_empty(), "{} de {} renomes falharam:\n{}", failures.len(), renamed_count + failures.len(), failures.join("\n"));
    assert!(renamed_count > 300, "poucos renomes: {renamed_count}");
}

#[test]
fn renaming_broken_programs_never_panics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut texts: Vec<String> = Vec::new();
    for e in std::fs::read_dir(root.join("tests/programs/err")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "lav") {
            texts.push(std::fs::read_to_string(p).unwrap());
        }
    }
    // O corpus cortado no meio de uma linha, como fica enquanto se digita.
    for path in corpus().iter().step_by(3) {
        let src = std::fs::read_to_string(path).unwrap();
        for cut in [src.len() / 3, src.len() / 2, 2 * src.len() / 3] {
            let cut = (0..=cut).rev().find(|&i| src.is_char_boundary(i)).unwrap();
            texts.push(src[..cut].to_string());
        }
    }
    for src in &texts {
        let a = lavra::analyze(src);
        for d in &a.tables.defs {
            let offset = d.span.end.saturating_sub(1);
            let _ = prepare(src, offset);
            for new in ["a zabumba", "o zimbro", "zabumbar", "z9", "valor", ""] {
                if let Ok(edits) = rename(src, offset, new) {
                    let _ = apply(src, &edits);
                }
            }
        }
    }
}
