//! Renomear um nome do programa em todos os lugares e em todas as formas, com a
//! morfologia da língua.
//!
//! - **Função**: cada uso na forma em que foi escrito (1ª pessoa, infinitivo, futuro do
//!   subjuntivo, infinitivo pessoal, o adjetivo concordando com o sujeito), e a 1ª
//!   pessoa declarada no cabeçalho ("Para trovejar (trovejo)").
//! - **Nome, tipo ou caso**: a palavra, e o artigo ou a contração quando o gênero muda
//!   ("do saldo" vira "da reserva", "uma conta" vira "um cofre"). O número fica: um nome
//!   no plural é coleção, e trocar o número mudaria o programa.
//! - **O mesmo nome em outra declaração**: o "Se houver apelido" e os campos que um
//!   braço de "Conforme" expõe são o mesmo nome e vão juntos. "Para cada nota" cita "as
//!   notas" pelo singular: renomear a coleção renomeia o item; renomear só o item deixa a
//!   coleção explícita ("Para cada avaliação das notas").
//!
//! O que concorda com o nome sem citá-lo ("As compras começam vazias", "Se a luz for
//! acesa", o "ambos" do cabeçalho) se ajusta com as correções seguras do próprio
//! compilador, aplicadas só ao que o rename fez aparecer e nunca ao que ele escreveu. O
//! ordinal que tinha o gênero da coleção ("a primeira das notas") vai junto, porque
//! a língua não o confere.
//!
//! No fim, o texto renomeado é analisado de novo: o rename é recusado se aparecer erro
//! que não havia ou se algum uso passar a apontar para outro nome.

use crate::Analysis;
use crate::ast::{FuncSig, SigKind, VerbForm};
use crate::diagnostics::{Category, Edit};
use crate::fix::{self, Edited};
use crate::lexer::{Token, TokenKind};
use crate::morphology::{self, Gender, Number};
use crate::span::Span;
use crate::tables::{DefKind, Form, Use};
use std::collections::{BTreeMap, HashMap};

/// O que se renomeia numa posição: a palavra sob o cursor e o nome como o editor o
/// propõe ("o saldo", "render", "uma conta").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub span: Span,
    pub placeholder: String,
}

/// `text` com a maiúscula do começo de `written` ("O" vira "A", não "a").
fn match_case(written: &str, text: &str) -> String {
    if written.chars().next().is_some_and(char::is_uppercase) {
        let mut c = text.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    } else {
        text.to_string()
    }
}

/// O mesmo artigo (ou contração com artigo) no gênero `g`: "do" vira "da", "uma" vira
/// "um", "à" vira "ao".
fn reinflect_article(written: &str, g: Gender) -> Option<String> {
    let lower = written.to_lowercase();
    let (prep, art) = match morphology::split_contraction(&lower) {
        Some((_, p, a)) => (Some(p), a.to_string()),
        None => (None, lower),
    };
    let a = morphology::article(&art)?;
    let new_art = if a.definite { morphology::definite_article(g, a.number) } else { morphology::indefinite_article(g, a.number) };
    let text = match prep {
        Some(p) => morphology::join_prep(p, new_art),
        None => new_art.to_string(),
    };
    Some(match_case(written, &text))
}

/// O nome na posição, se é do programa.
fn target_def(a: &Analysis, offset: u32) -> Result<usize, String> {
    let def = a.tables.def_at(offset).ok_or("Aqui não há nome do programa para renomear.")?;
    let d = &a.tables.defs[def];
    if d.kind == DefKind::Result {
        return Err("\"O resultado\" é da língua e não se renomeia.".into());
    }
    if let Some(lib) = crate::builtins::library_at(d.span.start) {
        return Err(format!("\"{}\" vem da biblioteca de {} e não se renomeia.", d.name, lib.name));
    }
    Ok(def)
}

fn sig_of(a: &Analysis, def: usize) -> Option<&FuncSig> {
    let span = a.tables.defs[def].span;
    a.ast.sigs.iter().find(|s| !s.is_main && s.name_span == span)
}

/// Uma ocorrência escrita de um nome: a palavra e o artigo logo antes dela (índices em
/// `Analysis::tokens`).
#[derive(Clone, Copy, Debug)]
struct Occurrence {
    word: usize,
    article: Option<usize>,
    form: Form,
}

/// A ocorrência num trecho (a declaração ou um uso) que termina na palavra do nome.
/// Trechos que não terminam no nome (o braço inteiro de um "Conforme") não são ocorrência.
fn occurrence(a: &Analysis, span: Span, form: Form, fits: impl Fn(&str) -> bool) -> Option<Occurrence> {
    let word = a.tokens.iter().rposition(|t| {
        t.span.end == span.end && t.span.start >= span.start && !t.span.is_empty() && t.contraction.is_none() && t.word().is_some_and(&fits)
    })?;
    let article = word
        .checked_sub(1)
        .filter(|&k| a.tokens[k].word().and_then(morphology::article).is_some() && a.tokens[k].contraction.is_none_or(|c| c.part == 2));
    Some(Occurrence { word, article, form })
}

/// As ocorrências de uma definição: a própria declaração e cada uso, sem repetir.
fn occurrences(a: &Analysis, def: usize) -> Vec<Occurrence> {
    let d = &a.tables.defs[def];
    let name = d.name.clone();
    let plural = morphology::pluralize(&name);
    let mut out: Vec<Occurrence> = Vec::new();
    let mut push = |o: Option<Occurrence>| {
        if let Some(o) = o
            && !out.iter().any(|x| x.word == o.word)
        {
            out.push(o);
        }
    };
    push(occurrence(a, d.span, Form::Name, |w| w == name));
    for r in a.tables.refs_of(def) {
        let o = match r.form {
            Form::Name => occurrence(a, r.span, r.form, |w| w == name),
            Form::Plural => occurrence(a, r.span, r.form, |w| w == plural),
            Form::Singular => occurrence(a, r.span, r.form, |w| morphology::pluralize(w) == name),
            Form::Verb(_) => occurrence(a, r.span, r.form, |_| true),
        };
        push(o);
    }
    out
}

/// O nome novo de um substantivo, de um tipo ou de um caso.
struct NewName {
    word: String,
    gender: Option<Gender>,
    number: Option<Number>,
    symbol: bool,
}

fn parse_name(new: &str) -> Result<NewName, String> {
    let words: Vec<String> = new.split_whitespace().map(str::to_lowercase).collect();
    let valid = |w: &str| w.chars().all(|c| c.is_alphabetic() || c == '-') && w.chars().next().is_some_and(char::is_alphabetic);
    match words.as_slice() {
        [w] if morphology::is_symbol(w) => Ok(NewName { word: w.clone(), gender: None, number: None, symbol: true }),
        [w] if valid(w) => Ok(NewName { word: w.clone(), gender: None, number: None, symbol: false }),
        [art, w] if valid(w) => {
            let a = morphology::article(art).ok_or_else(|| format!("\"{art}\" não é artigo: escreva \"o {w}\" ou \"a {w}\"."))?;
            Ok(NewName { word: w.clone(), gender: Some(a.gender), number: Some(a.number), symbol: false })
        }
        _ => Err(format!("\"{}\" não é um nome: escreva uma palavra com o artigo, como \"a reserva\".", new.trim())),
    }
}

/// As edições do rename, sobre o texto original, sem repetir trecho.
struct Plan<'a> {
    a: &'a Analysis,
    src: &'a str,
    edits: BTreeMap<(u32, u32), String>,
}

impl Plan<'_> {
    fn written(&self, span: Span) -> &str {
        &self.src[span.start as usize..span.end as usize]
    }

    fn put(&mut self, span: Span, text: &str) {
        let text = match_case(self.written(span), text);
        if self.written(span) != text {
            self.edits.insert((span.start, span.end), text);
        }
    }

    fn token(&self, i: usize) -> &Token {
        &self.a.tokens[i]
    }

    /// O artigo de uma ocorrência no gênero novo, se ele concordava com o gênero velho.
    fn article(&mut self, o: &Occurrence, old: Gender, new: Gender) {
        let Some(k) = o.article.filter(|_| old != new) else { return };
        let t = self.token(k).clone();
        if t.word().and_then(morphology::article).is_some_and(|a| a.gender == old)
            && let Some(text) = reinflect_article(self.written(t.span), new)
        {
            self.put(t.span, &text);
        }
    }

    /// "a primeira das notas": o ordinal e o artigo dele seguem a coleção, quando tinham o
    /// gênero dela.
    fn ordinal(&mut self, o: &Occurrence, old: Gender, new: Gender) {
        let Some(k) = o.article.filter(|_| old != new) else { return };
        let contraction = self.token(k).contraction;
        if contraction.is_none_or(|c| c.written.strip_prefix('d').is_none()) {
            return;
        }
        // A preposição da contração é o token de antes, com o mesmo trecho.
        let Some(ord) = k.checked_sub(2) else { return };
        let ord_tok = self.token(ord).clone();
        let Some(w) = ord_tok.word() else { return };
        if !morphology::ordinal(w).is_some_and(|(_, g, _)| g == old) {
            return;
        }
        let Some(art) = ord.checked_sub(1) else { return };
        let art_tok = self.token(art).clone();
        if !art_tok.word().and_then(morphology::article).is_some_and(|a| a.gender == old) {
            return;
        }
        if let Some(text) = morphology::inflect_ordinal(w, new) {
            self.put(ord_tok.span, &text);
        }
        if let Some(text) = reinflect_article(self.written(art_tok.span), new) {
            self.put(art_tok.span, &text);
        }
    }

    /// Os nomes que são um só com `def`: declarados no mesmo trecho (os campos de um caso
    /// e os nomes que cada braço de "Conforme" expõe), ou ligados pelo "Se houver".
    fn family(&self, def: usize) -> Vec<usize> {
        let t = &self.a.tables;
        let mut out = vec![def];
        let mut i = 0;
        while i < out.len() {
            let d = out[i];
            let span = t.defs[d].span;
            let mut more: Vec<usize> = Vec::new();
            for (j, x) in t.defs.iter().enumerate() {
                if x.kind != DefKind::Result && x.kind != DefKind::Function && x.span == span {
                    more.push(j);
                }
            }
            for r in t.refs.iter().filter(|r| r.form == Form::Name) {
                // O "Se houver apelido" declara o apelido de dentro onde cita o de fora.
                if r.def == d {
                    more.extend(t.defs.iter().enumerate().filter(|(_, x)| x.kind == DefKind::Local && x.span == r.span).map(|(j, _)| j));
                }
                if r.span == span && r.def != d && t.defs[r.def].kind != DefKind::Function {
                    more.push(r.def);
                }
            }
            for j in more {
                if !out.contains(&j) {
                    out.push(j);
                }
            }
            i += 1;
        }
        out
    }

    /// Um substantivo (variável, constante, parâmetro, campo), um tipo ou um caso.
    fn name(&mut self, def: usize, new: &str, is_type: bool) -> Result<(), String> {
        let t = &self.a.tables;
        let d = t.defs[def].clone();
        let new = parse_name(new)?;
        let old_symbol = morphology::is_symbol(&d.name) && !d.display.contains(' ');
        if new.symbol != old_symbol {
            return Err(if old_symbol {
                format!("\"{}\" é símbolo, e símbolo troca por outro símbolo (uma letra e algarismos).", d.name)
            } else {
                format!("\"{}\" tem artigo, e troca por outro nome com artigo, como \"{}\".", d.display, d.display)
            });
        }
        if let Some(n) = new.number.filter(|n| *n != d.number) {
            let _ = n;
            return Err(format!(
                "O nome novo fica no {}, como \"{}\": trocar o número mudaria o programa.",
                d.number.describe(),
                d.display
            ));
        }
        if is_type && new.number == Some(Number::Plural) {
            return Err("Um tipo se nomeia no singular: \"uma conta\".".into());
        }
        let gender = match (new.symbol, new.gender) {
            (true, _) => d.gender,
            (false, Some(g)) => g,
            (false, None) => morphology::gender_by_ending(&new.word).ok_or_else(|| {
                let (o, a) = if d.number == Number::Plural { ("os", "as") } else { ("o", "a") };
                format!("Diga o gênero com o artigo: \"{o} {}\" ou \"{a} {}\".", new.word, new.word)
            })?,
        };
        // O item de "Para cada" tem o gênero da coleção quando ela é um nome no plural.
        if gender != d.gender
            && let Some(coll) = self.collection_of_item(def)
            && t.defs[coll].gender != gender
        {
            let c = &t.defs[coll];
            let g = c.gender.describe();
            return Err(format!("O item de \"Para cada\" tem o gênero da coleção, \"{}\" ({g}): o nome novo também é {g}.", c.display));
        }
        let family = self.family(def);
        for &member in &family {
            self.adjective_subject(member, gender);
            let m = &t.defs[member];
            let old_gender = m.gender;
            for o in occurrences(self.a, member) {
                let word_span = self.token(o.word).span;
                match o.form {
                    Form::Name => {
                        // O item de um "Para cada nota" que só este nome tem: a coleção
                        // continua com o nome dela e passa a ser dita.
                        if let Some(c) = t.refs.iter().find(|r| r.form == Form::Singular && r.span == word_span && !family.contains(&r.def))
                        {
                            let coll = &t.defs[c.def];
                            let of = morphology::join_prep("de", morphology::definite_article(coll.gender, Number::Plural));
                            self.put(word_span, &format!("{} {of} {}", new.word, coll.name));
                            continue;
                        }
                        self.put(word_span, &new.word);
                        // "Para cada herdeiro dos herdeiros": o item que é o singular da
                        // coleção vai junto com ela.
                        if let Some(item) = self.derived_item(&o, &m.name) {
                            self.rename_item(item, &new.word, gender, None);
                        }
                    }
                    Form::Plural => self.put(word_span, &morphology::pluralize(&new.word)),
                    Form::Singular => {
                        let items: Vec<usize> = t
                            .defs
                            .iter()
                            .enumerate()
                            .filter(|(_, x)| x.kind == DefKind::Local && x.span == word_span)
                            .map(|(i, _)| i)
                            .collect();
                        for item in items {
                            self.rename_item(item, &new.word, gender, Some(word_span));
                        }
                        continue;
                    }
                    Form::Verb(_) => continue,
                }
                self.article(&o, old_gender, gender);
                self.ordinal(&o, old_gender, gender);
            }
        }
        Ok(())
    }

    /// O item de um "Para cada" que deriva da coleção renomeada (o singular dela) vai para
    /// o singular do nome novo, com o gênero dela. Sem singular a derivar, a coleção do
    /// "Para cada nota" (`implicit`) passa a ser dita e o item guarda o nome.
    fn rename_item(&mut self, item: usize, plural: &str, gender: Gender, implicit: Option<Span>) {
        let Some(singular) = morphology::singularize(plural) else {
            if let Some(span) = implicit {
                let of = morphology::join_prep("de", morphology::definite_article(gender, Number::Plural));
                let written = self.written(span).to_string();
                self.put(span, &format!("{written} {of} {plural}"));
            }
            return;
        };
        let old = self.a.tables.defs[item].gender;
        for o in occurrences(self.a, item) {
            let span = self.token(o.word).span;
            self.put(span, &singular);
            self.article(&o, old, gender);
        }
    }

    /// "Para cada herdeiro dos herdeiros": o item escrito antes da coleção, quando ele é o
    /// singular dela.
    fn derived_item(&self, o: &Occurrence, collection: &str) -> Option<usize> {
        let k = o.article?;
        let item = k.checked_sub(2)?;
        let tokens = &self.a.tokens;
        if !(tokens[k].contraction.is_some_and(|c| c.written.starts_with('d')) && item > 0 && tokens[item - 1].is_word("cada")) {
            return None;
        }
        let w = tokens[item].word()?;
        if morphology::pluralize(w) != collection {
            return None;
        }
        let span = tokens[item].span;
        self.a.tables.defs.iter().position(|d| d.kind == DefKind::Local && d.span == span)
    }

    /// A coleção de que `def` é o item num "Para cada", quando ela é um nome no plural.
    fn collection_of_item(&self, def: usize) -> Option<usize> {
        let t = &self.a.tables;
        let d = &t.defs[def];
        if d.kind != DefKind::Local {
            return None;
        }
        let tokens = &self.a.tokens;
        let w = tokens.iter().position(|tk| tk.span == d.span)?;
        if !w.checked_sub(1).is_some_and(|k| tokens[k].is_word("cada")) {
            return None;
        }
        if let Some(r) = t.refs.iter().find(|r| r.form == Form::Singular && r.span == d.span) {
            return Some(r.def);
        }
        let next = tokens.get(w + 1)?;
        t.refs
            .iter()
            .find(|r| r.span.start == next.span.start && r.form == Form::Name && t.defs[r.def].number == Number::Plural)
            .map(|r| r.def)
    }

    /// Se `def` é o sujeito de um predicado adjetivo ("Para o terreno ser quadrado"), o
    /// adjetivo do cabeçalho acompanha o gênero novo: é dele que sai a forma de cada uso.
    fn adjective_subject(&mut self, def: usize, gender: Gender) {
        let d = &self.a.tables.defs[def];
        if d.kind != DefKind::Param || d.gender == gender {
            return;
        }
        let found = self.a.ast.sigs.iter().find_map(|s| match &s.kind {
            SigKind::Adjective { adjective, base_gender, span } if s.params.first().is_some_and(|p| p.name.span == d.span) => {
                Some((adjective.clone(), *base_gender, *span))
            }
            _ => None,
        });
        if let Some((adjective, base, span)) = found {
            self.put(span, &morphology::inflect_adjective(&adjective, base, gender, d.number));
        }
    }

    /// Uma função: o verbo em cada forma em que aparece.
    fn function(&mut self, def: usize, new: &str) -> Result<(), String> {
        let sig = sig_of(self.a, def).ok_or("Esta função não tem cabeçalho para renomear.")?.clone();
        let refs = self.a.tables.refs_of(def);
        if let SigKind::Adjective { base_gender, span, .. } = &sig.kind {
            let adjective = new.trim().to_lowercase();
            if adjective.is_empty() || !adjective.chars().all(|c| c.is_alphabetic() || c == '-') {
                return Err(format!("\"{}\" não é um adjetivo: escreva como no cabeçalho, \"par\".", new.trim()));
            }
            let subject = sig.params.first().map(|p| (p.name.gender(), p.name.number()));
            let (g, n) = subject.unwrap_or((*base_gender, Number::Singular));
            self.put(*span, &morphology::inflect_adjective(&adjective, *base_gender, g, n));
            for r in refs {
                if let Form::Verb(VerbForm::Adjective(g, n)) = r.form {
                    self.put(r.span, &morphology::inflect_adjective(&adjective, *base_gender, g, n));
                }
            }
            return Ok(());
        }
        let lower = new.trim().to_lowercase();
        let (infinitive, declared) = match lower.split_once('(') {
            Some((inf, rest)) => (inf.trim().to_string(), Some(rest.trim_end().trim_end_matches(')').trim().to_string())),
            None => (lower.clone(), None),
        };
        if infinitive.contains(char::is_whitespace) || !morphology::verbs::is_infinitive(&infinitive) {
            return Err(format!("\"{}\" não é um verbo no infinitivo: escreva como no cabeçalho, \"aplicar\".", new.trim()));
        }
        let first_person = match declared.clone().or_else(|| morphology::first_person(&infinitive)) {
            Some(f) => f,
            None if sig.kind == SigKind::Action => {
                return Err(format!(
                    "\"{infinitive}\" não tem 1ª pessoa do presente, e é ela que executa: declare a forma, \"{infinitive} (...)\"."
                ));
            }
            None => infinitive.clone(),
        };
        self.put(sig.name_span, &infinitive);
        // A 1ª pessoa declarada no cabeçalho: troca, some (o verbo novo tem a regular) ou
        // aparece (o verbo novo pede).
        match (sig.first_person_span, &declared) {
            (Some(span), Some(f)) => self.put(span, f),
            (Some(span), None) => {
                let close = self.a.tokens.iter().find(|t| t.span.start >= span.end && matches!(t.kind, TokenKind::Punct(')')));
                if let Some(close) = close {
                    let gap = Span::new(sig.name_span.end as usize, close.span.end as usize);
                    self.edits.insert((gap.start, gap.end), String::new());
                }
            }
            (None, Some(f)) => {
                self.edits.insert((sig.name_span.end, sig.name_span.end), format!(" ({f})"));
            }
            (None, None) => {}
        }
        for r in refs {
            let text = match r.form {
                Form::Verb(VerbForm::FirstPerson) => first_person.clone(),
                Form::Verb(VerbForm::Infinitive) => infinitive.clone(),
                Form::Verb(VerbForm::Subjunctive(n)) => morphology::future_subjunctive(&infinitive, n),
                Form::Verb(VerbForm::PersonalInfinitive(n)) => morphology::personal_infinitive(&infinitive, n),
                _ => continue,
            };
            self.put(r.span, &text);
        }
        Ok(())
    }
}

/// O nome na posição, e como o editor o propõe.
pub fn prepare(src: &str, offset: u32) -> Result<Target, String> {
    let a = crate::analyze(src);
    let def = target_def(&a, offset)?;
    let d = &a.tables.defs[def];
    let placeholder = match (d.kind, sig_of(&a, def)) {
        (DefKind::Function, Some(s)) => match &s.kind {
            SigKind::Adjective { adjective, .. } => adjective.clone(),
            _ if s.first_person_span.is_some() => format!("{} ({})", s.infinitive, s.first_person),
            _ => s.infinitive.clone(),
        },
        _ => d.display.clone(),
    };
    let contains = |s: Span| s.start <= offset && offset <= s.end;
    let span = if d.kind == DefKind::Function {
        a.tables.refs_of(def).iter().map(|r| r.span).chain([d.span]).find(|s| contains(*s))
    } else {
        a.tables.refs_of(def).iter().map(|r| r.span).chain([d.span]).filter(|s| contains(*s)).min_by_key(|s| s.end - s.start)
    };
    // Só a palavra, sem o artigo.
    let span =
        span.and_then(|s| a.tokens.iter().rfind(|t| t.span.end == s.end && t.span.start >= s.start).map(|t| t.span)).unwrap_or(d.span);
    Ok(Target { span, placeholder })
}

/// As edições que renomeiam o nome na posição para `new`, sobre o texto `src`.
pub fn rename(src: &str, offset: u32, new: &str) -> Result<Vec<Edit>, String> {
    let a = crate::analyze(src);
    let def = target_def(&a, offset)?;
    let mut plan = Plan { a: &a, src, edits: BTreeMap::new() };
    match a.tables.defs[def].kind {
        DefKind::Function => plan.function(def, new)?,
        DefKind::Record | DefKind::Enum | DefKind::Variant => plan.name(def, new, true)?,
        _ => plan.name(def, new, false)?,
    }
    let ours: Vec<Edit> = plan.edits.into_iter().map(|((s, e), text)| Edit { span: Span::new(s as usize, e as usize), text }).collect();
    if ours.is_empty() {
        return Ok(Vec::new());
    }
    let mut edited = Edited::new(src);
    edited.apply(&ours);
    // O que concorda com o nome e o rename não escreveu: as correções seguras dos erros de
    // concordância que não havia antes.
    let before: Vec<(&str, u32)> = a.diagnostics.iter().filter(|d| d.is_error()).filter_map(|d| Some((d.code?, d.span.start))).collect();
    fix::fix_with(&mut edited, |d, ed| {
        let ours_now: Vec<(u32, u32)> = ours.iter().map(|o| (ed.map(o.span.start), ed.map(o.span.start) + o.text.len() as u32)).collect();
        let touches_ours = d
            .fixes
            .iter()
            .flat_map(|f| &f.edits)
            .any(|e| ours_now.iter().any(|&(s, t)| e.span.start < t && s < e.span.end.max(e.span.start + 1)));
        d.category == Category::Agreement && !touches_ours && !before.iter().any(|(c, p)| d.code == Some(*c) && ed.map(*p) == d.span.start)
    });
    verify(&a, &edited, new)?;
    Ok(edited.edits().to_vec())
}

/// O texto renomeado não pode ter erro novo, e cada uso tem de apontar para o mesmo nome
/// de antes.
fn verify(before: &Analysis, edited: &Edited, new: &str) -> Result<(), String> {
    let text = edited.text();
    let after = crate::analyze(&text);
    let mut count: HashMap<&str, i64> = HashMap::new();
    for d in before.diagnostics.iter().filter(|d| d.is_error()) {
        *count.entry(d.code.unwrap_or("")).or_default() += 1;
    }
    for d in after.diagnostics.iter().filter(|d| d.is_error()) {
        let c = count.entry(d.code.unwrap_or("")).or_default();
        *c -= 1;
        if *c < 0 {
            let code = d.code.map(|c| format!(" [{c}]")).unwrap_or_default();
            return Err(format!("Renomeado assim, o programa teria um erro{code}: {}", d.message));
        }
    }
    let lines = crate::span::LineIndex::new(&text);
    let identity = |a: &Analysis, def: usize| a.tables.defs[def].span.start;
    // O que vem de uma biblioteca fica fora do texto e não se move com as edições.
    let moved = |pos: u32| if pos >= crate::builtins::LIBRARY_BASE { pos } else { edited.map(pos) };
    for r in before.tables.refs.iter().filter(|r| r.use_ != Use::Mention) {
        let at = edited.map(r.span.start);
        let expected = moved(identity(before, r.def));
        let now: Vec<_> = after.tables.refs.iter().filter(|x| x.span.start == at).collect();
        if !now.is_empty() && !now.iter().any(|x| identity(&after, x.def) == expected) {
            let line = lines.line(at as usize) + 1;
            return Err(format!(
                "Renomear para \"{}\" mudaria o sentido do programa: na linha {line}, um uso passaria a ser de outro nome.",
                new.trim()
            ));
        }
    }
    Ok(())
}
