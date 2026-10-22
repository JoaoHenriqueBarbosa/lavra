//! Dados: registros ("Uma conta é um registro de: ..."), lugares com projeção ("o nome
//! do titular da conta", "o item i das notas", "a última das notas"), coleções e as
//! operações nominais ("a soma de x e y").

use super::Parser;
use crate::ast::*;
use crate::codes::*;
use crate::diagnostics::{Diagnostic, Result, err};
use crate::lexer::TokenKind;
use crate::lexicon::{self, NominalOp};
use crate::morphology::{self, Gender, Number};
use crate::span::Span;

pub(super) fn is_nominal_word(w: &str) -> bool {
    lexicon::nominal(w).is_some()
}

impl Parser {
    // ------------------------------------------------------------- registros

    /// Pré-varredura dos registros: primeiro os nomes (um registro pode citar outro
    /// definido depois), depois os campos.
    /// Pré-varredura dos registros e enumerações, antes dos cabeçalhos e dos corpos.
    /// Uma definição com erro é relatada, fica com o que se leu até ali (marcada como
    /// quebrada) e é pulada inteira; as outras seguem.
    pub(super) fn prescan_records(&mut self) {
        let mut starts = Vec::new();
        let mut enum_starts = Vec::new();
        let mut depth = 0usize;
        let mut at_start = true;
        for k in 0..self.toks.len() {
            let t = &self.toks[k];
            match t.kind {
                TokenKind::Indent => depth += 1,
                TokenKind::Dedent => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0
                && at_start
                && t.word().and_then(morphology::article).is_some_and(|a| !a.definite)
                && self.toks.get(k + 2).is_some_and(|n| n.is_word("é"))
                && self.toks.get(k + 4).is_some_and(|n| n.is_word("registro"))
            {
                starts.push(k);
            } else if depth == 0
                && at_start
                && t.word().and_then(morphology::article).is_some_and(|a| !a.definite)
                && self.toks.get(k + 2).is_some_and(|n| n.is_word("é"))
                && self.toks.get(k + 4).is_some_and(|n| n.is_word("destas") || n.is_word("destes"))
            {
                enum_starts.push(k);
            }
            at_start = matches!(t.kind, TokenKind::Newline | TokenKind::Dedent | TokenKind::Indent);
        }
        // Nomes das enumerações antes dos campos de qualquer registro.
        let enum_starts: Vec<usize> = enum_starts.into_iter().filter(|&k| self.define_type(k, true)).collect();
        let starts: Vec<usize> = starts.into_iter().filter(|&k| self.define_type(k, false)).collect();
        for k in starts {
            let id = self.vocab.records[self.toks[k + 1].word().unwrap()];
            if let Err(d) = self.record_body(k, id) {
                self.give_up_definition(k, d);
                self.prog.records[id].broken = true;
            }
        }
        for k in enum_starts {
            let id = self.vocab.enums[self.toks[k + 1].word().unwrap()];
            if let Err(d) = self.enum_body(k, id) {
                self.give_up_definition(k, d);
                self.prog.enums[id].broken = true;
            }
        }
        self.pos = 0;
    }

    /// Registra o nome do tipo que começa em `k`. Com erro, relata, pula a definição e
    /// devolve falso.
    fn define_type(&mut self, k: usize, is_enum: bool) -> bool {
        let art_tok = self.toks[k].clone();
        let name_tok = self.toks[k + 1].clone();
        let art = morphology::article(art_tok.word().unwrap()).unwrap();
        let checked = (|| {
            let Some(name) = name_tok.word().map(str::to_string) else {
                let what = if is_enum { "Esperava o nome do tipo." } else { "Esperava o nome do registro." };
                return err(Diagnostic::new(L0200, what, name_tok.span));
            };
            if art.number != Number::Singular {
                let what = if is_enum {
                    "Um tipo se define no singular: \"Uma forma é uma destas:\"."
                } else {
                    "Um tipo de registro se define no singular: \"Uma conta é um registro de:\"."
                };
                return err(Diagnostic::new(L0201, what, art_tok.span));
            }
            if is_enum && (self.vocab.records.contains_key(&name) || self.vocab.enums.contains_key(&name)) {
                return err(Diagnostic::new(L0202, format!("O tipo \"{name}\" já foi definido."), name_tok.span));
            }
            if !is_enum && self.vocab.records.contains_key(&name) {
                return err(Diagnostic::new(L0202, format!("O registro \"{name}\" já foi definido."), name_tok.span));
            }
            Ok(name)
        })();
        let name = match checked {
            Ok(name) => name,
            Err(d) => {
                self.give_up_definition(k, d);
                return false;
            }
        };
        let plural = morphology::pluralize(&name);
        let span = art_tok.span.to(name_tok.span);
        if is_enum {
            let id = self.prog.enums.len();
            self.vocab.enums.insert(name.clone(), id);
            self.vocab.enum_plurals.insert(plural.clone(), id);
            self.prog.enums.push(EnumDef { word: name, plural, gender: art.gender, span, variants: Vec::new(), broken: false });
        } else {
            let id = self.prog.records.len();
            self.vocab.records.insert(name.clone(), id);
            self.vocab.record_plurals.insert(plural.clone(), id);
            self.prog.records.push(RecordDef { word: name, plural, gender: art.gender, span, fields: Vec::new(), broken: false });
        }
        true
    }

    /// Relata o erro de uma definição e pula até o fim dela.
    fn give_up_definition(&mut self, k: usize, d: Box<Diagnostic>) {
        let end = self.definition_end(k);
        self.report_in(k, end, d);
        self.skip_spans.insert(k, end);
    }

    /// "Uma conta é um registro de:" seguido dos campos recuados.
    fn record_body(&mut self, k: usize, id: usize) -> Result<()> {
        self.pos = k + 2;
        self.expect_word("é")?;
        let um = self.bump();
        if !um.is_word("um") {
            return err(Diagnostic::new(L0203, "\"registro\" é masculino: \"é um registro de\".", um.span));
        }
        self.expect_word("registro")?;
        self.expect_word("de")?;
        self.expect_punct(':', "os dois-pontos")?;
        self.skip_newlines();
        if !matches!(self.peek().kind, TokenKind::Indent) {
            return Err(self.expected("os campos recuados na linha de baixo"));
        }
        self.bump();
        loop {
            let name = self.name_ref()?;
            if self.prog.records[id].fields.iter().any(|f| f.name.word == name.word) {
                return err(Diagnostic::new(L0204, format!("O campo \"{}\" aparece duas vezes.", name.word), name.span));
            }
            if is_nominal_word(&name.word) || name.word == "item" {
                return err(Diagnostic::new(
                    L0205,
                    format!("\"{}\" é nome de operação da língua e não serve de campo.", name.word),
                    name.span,
                ));
            }
            self.expect_punct(',', "a vírgula entre o campo e o tipo")?;
            let ann = self.aposto()?;
            self.vocab.field_names.insert(name.word.clone());
            self.prog.records[id].fields.push(FieldDef { name, ty: ann });
            if self.eat_punct(';') {
                self.skip_newlines();
                continue;
            }
            self.expect_punct('.', "o ponto final depois do último campo (ou ponto e vírgula entre campos)")?;
            break;
        }
        self.skip_newlines();
        if !matches!(self.peek().kind, TokenKind::Dedent | TokenKind::Eof) {
            return Err(self.expected("o fim do registro"));
        }
        if matches!(self.peek().kind, TokenKind::Dedent) {
            self.bump();
        }
        self.skip_spans.insert(k, self.pos);
        Ok(())
    }

    /// "Uma forma é uma destas: um círculo, com o raio, um real; ...; um ponto."
    fn enum_body(&mut self, k: usize, id: usize) -> Result<()> {
        let gender = self.prog.enums[id].gender;
        self.pos = k + 2;
        self.expect_word("é")?;
        let art = self.bump();
        let these = self.bump();
        let (right_art, right_these) = match gender {
            Gender::Feminine => ("uma", "destas"),
            Gender::Masculine => ("um", "destes"),
        };
        if !art.is_word(right_art) || !these.is_word(right_these) {
            return err(Diagnostic::new(
                L0206,
                format!("\"{}\" é {}: \"é {right_art} {right_these}\".", self.prog.enums[id].word, gender.describe()),
                art.span.to(these.span),
            ));
        }
        self.expect_punct(':', "os dois-pontos")?;
        self.skip_newlines();
        if !matches!(self.peek().kind, TokenKind::Indent) {
            return Err(self.expected("as formas recuadas na linha de baixo"));
        }
        self.bump();
        loop {
            let at = self.bump();
            let Some(a) = at.word().and_then(morphology::article).filter(|a| !a.definite && a.number == Number::Singular) else {
                return err(Diagnostic::new(
                    L0207,
                    format!("Cada caso começa com artigo indefinido (\"um círculo\"), mas encontrei {}.", at.describe()),
                    at.span,
                ));
            };
            let wt = self.bump();
            let Some(word) = wt.word().map(str::to_string) else {
                return err(Diagnostic::new(L0208, "Esperava o nome do caso.", wt.span));
            };
            if self.vocab.variants.contains_key(&word) || self.vocab.records.contains_key(&word) || self.vocab.enums.contains_key(&word) {
                return err(Diagnostic::new(L0209, format!("\"{word}\" já é nome de outro tipo ou caso."), wt.span));
            }
            let position = self.prog.enums[id].variants.len();
            self.vocab.variants.insert(word.clone(), (id, position));
            self.prog.enums[id].variants.push(VariantDef { word, gender: a.gender, span: at.span.to(wt.span), fields: Vec::new() });
            if self.at_punct(',') && self.peek_at(1).is_word("com") {
                self.bump();
                self.bump();
                loop {
                    let name = self.name_ref()?;
                    if name.is_symbol() {
                        return err(Diagnostic::new(L0210, "Campo leva nome com artigo (\"o raio\").", name.span));
                    }
                    let fields = &self.prog.enums[id].variants[position].fields;
                    if fields.iter().any(|f| f.name.word == name.word) {
                        return err(Diagnostic::new(L0204, format!("O campo \"{}\" aparece duas vezes.", name.word), name.span));
                    }
                    self.expect_punct(',', "a vírgula entre o campo e o tipo")?;
                    let ann = self.aposto()?;
                    self.vocab.field_names.insert(name.word.clone());
                    self.prog.enums[id].variants[position].fields.push(FieldDef { name, ty: ann });
                    if self.at_punct(',') && self.peek_at(1).is_word("e") {
                        self.bump();
                        self.bump();
                        continue;
                    }
                    if self.at_word("e") && self.definite_article_at(1).is_some() {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            if self.eat_punct(';') {
                self.skip_newlines();
                continue;
            }
            self.expect_punct('.', "o ponto final depois do último caso (ou ponto e vírgula entre casos)")?;
            break;
        }
        self.skip_newlines();
        if matches!(self.peek().kind, TokenKind::Dedent) {
            self.bump();
        }
        self.skip_spans.insert(k, self.pos);
        Ok(())
    }

    /// Tipo de registro no número pedido; devolve o nome do registro e o gênero.
    pub(super) fn record_type_word(&self, w: &str, number: Number) -> Option<(String, Gender)> {
        let rec = match number {
            Number::Singular => self.vocab.records.get(w),
            Number::Plural => self.vocab.record_plurals.get(w),
        };
        if let Some(id) = rec {
            let r = &self.prog.records[*id];
            return Some((r.word.clone(), r.gender));
        }
        let id = match number {
            Number::Singular => self.vocab.enums.get(w),
            Number::Plural => self.vocab.enum_plurals.get(w),
        }?;
        let e = &self.prog.enums[*id];
        Some((e.word.clone(), e.gender))
    }

    // --------------------------------------------------------------- lugares

    /// Um segmento de lugar pode ser seguido de "de" + outro lugar?
    fn projects(&self, seg: &Segment) -> bool {
        match seg {
            Segment::Name(n) => self.vocab.field_names.contains(&n.word) && !n.is_symbol(),
            Segment::Index(..) | Segment::Ordinal { .. } => true,
        }
    }

    /// Lugar: "a carta", "x", "o saldo da conta", "o item i das notas", "a última das
    /// notas". O "de" genitivo se lê da direita para a esquerda.
    pub(super) fn place(&mut self) -> Result<Place> {
        let start = self.peek().span;
        let mut segs = vec![self.segment()?];
        while self.at_word("de") && self.projects(segs.last().unwrap()) {
            let save = self.pos;
            self.bump();
            if self.definite_article_at(0).is_none() && !self.peek().word().is_some_and(morphology::is_symbol) {
                self.pos = save;
                break;
            }
            segs.push(self.segment()?);
        }
        let root = match segs.pop().unwrap() {
            Segment::Name(n) => n,
            Segment::Index(_, span) | Segment::Ordinal { span, .. } => {
                return err(Diagnostic::new(L0211, "Falta a coleção: \"o item 2 das notas\", \"a última das notas\".", span));
            }
        };
        let proj = segs
            .into_iter()
            .rev()
            .map(|s| match s {
                Segment::Name(n) => Proj::Field(n),
                Segment::Index(e, span) => Proj::Index(Box::new(e), span),
                Segment::Ordinal { position, gender, span } => Proj::Ordinal { position, gender, span },
            })
            .collect();
        Ok(Place { root, proj, span: start.to(self.prev_span()) })
    }

    fn segment(&mut self) -> Result<Segment> {
        if self.definite_article_at(0).is_some() {
            let at = self.peek().clone();
            let w = self.peek_at(1).clone();
            if w.is_word("item") {
                self.check_contraction()?;
                self.bump();
                self.bump();
                let e = self.additive()?;
                let span = at.span.to(self.prev_span());
                return Ok(Segment::Index(e, span));
            }
            if let Some((position, g, n)) = w.word().and_then(morphology::ordinal) {
                let a = self.definite_article_at(0).unwrap();
                if a.gender != g || a.number != n || n != Number::Singular {
                    let right = format!("{} {}", morphology::definite_article(g, Number::Singular), morphology::ordinal_word(position, g));
                    return err(Diagnostic::new(L0212, format!("O ordinal concorda com o artigo: \"{right}\"."), at.span.to(w.span)).fix(
                        format!("Escrever \"{right}\""),
                        at.span.to(w.span),
                        right.clone(),
                    ));
                }
                self.check_contraction()?;
                self.bump();
                self.bump();
                return Ok(Segment::Ordinal { position, gender: g, span: at.span.to(w.span) });
            }
        }
        Ok(Segment::Name(self.name_ref()?))
    }

    /// "o último", "a primeira", "o item 2": qual item sai de uma coleção.
    pub(super) fn which(&mut self) -> Result<Which> {
        match self.segment()? {
            Segment::Ordinal { position, span, .. } => Ok(Which::Ordinal { position, span }),
            Segment::Index(e, _) => Ok(Which::Item(e)),
            Segment::Name(n) => err(Diagnostic::new(
                L0213,
                format!("Esperava qual item (\"o último\", \"a primeira\", \"o item 2\"), mas encontrei \"{}\".", n.display()),
                n.span,
            )),
        }
    }

    // -------------------------------------------------------------- coleções

    /// Itens escritos por extenso: "7, 8,5 e 9".
    pub(super) fn item_list(&mut self) -> Result<Init> {
        let start = self.peek().span;
        let mut items = vec![self.expr()?];
        loop {
            let save = self.pos;
            let sep = self.eat_punct(',') || self.at_word("e");
            if self.at_word("e") {
                self.bump();
            }
            if sep && self.starts_value(0) && !self.subject_follows() {
                items.push(self.expr()?);
                continue;
            }
            self.pos = save;
            break;
        }
        let span = start.to(self.prev_span());
        Ok(if items.len() == 1 { Init::Expr(items.pop().unwrap()) } else { Init::List(items, span) })
    }

    /// Depois de "e": começa uma oração com sujeito ("e o saldo cresce 1")?
    fn subject_follows(&self) -> bool {
        let mut k = 0;
        loop {
            let t = self.peek_at(k);
            match &t.kind {
                TokenKind::Word(w) if super::subject_verb(w).is_some() => return true,
                TokenKind::Word(_) | TokenKind::Int(_) | TokenKind::Real(_) | TokenKind::Text(_) | TokenKind::Char(_) => k += 1,
                _ => return false,
            }
            if k > 8 {
                return false;
            }
        }
    }

    /// "vazio", "vazias": concorda com o sujeito.
    pub(super) fn empty_word(&mut self, subject: &NameRef) -> Result<Option<Span>> {
        let Some(w) = self.peek().word() else { return Ok(None) };
        let Some((g, n)) = morphology::adjective_form(w, "vazio") else { return Ok(None) };
        let t = self.bump();
        if g != subject.gender() || n != subject.number() {
            let right = morphology::inflect_adjective("vazio", Gender::Masculine, subject.gender(), subject.number());
            return err(Diagnostic::new(L0214, format!("\"{}\" concorda com {}: \"{right}\".", t.orig, subject.display()), t.span).fix(
                format!("Escrever \"{right}\""),
                t.span,
                right.clone(),
            ));
        }
        Ok(Some(t.span))
    }

    // ----------------------------------------------------- operações nominais

    /// "a soma de x e y", "o tamanho das notas", "o deslocamento de x por 2 à esquerda".
    pub(super) fn nominal(&mut self) -> Result<Option<Expr>> {
        let Some(art) = self.definite_article_at(0) else { return Ok(None) };
        let at = self.peek().clone();
        let wt = self.peek_at(1).clone();
        let Some(w) = wt.word() else { return Ok(None) };
        if self.vocab.field_names.contains(w) || !is_nominal_word(w) {
            return Ok(None);
        }
        let next = self.peek_at(2);
        if !(next.is_word("de") || next.is_word("entre")) {
            return Ok(None);
        }
        let w = w.to_string();
        let check_gender = |g: Gender| -> Result<()> {
            if art.gender != g || art.number != Number::Singular {
                let right = morphology::definite_article(g, Number::Singular);
                return err(Diagnostic::new(L0215, format!("\"{w}\" é {}: \"{right} {w}\".", g.describe()), at.span.to(wt.span)).fix(
                    format!("Escrever \"{right} {w}\""),
                    at.span.to(wt.span),
                    format!("{right} {w}"),
                ));
            }
            Ok(())
        };
        self.check_contraction()?;
        self.bump();
        self.bump();
        let nominal = lexicon::nominal(&w).expect("palavra nominal conferida acima");
        if nominal.op == NominalOp::Shift {
            check_gender(nominal.gender)?;
            self.expect_word("de")?;
            let value = self.nominal_operand()?;
            self.expect_word("por")?;
            let amount = self.nominal_operand()?;
            let dir = self.bump();
            let op = match dir.word() {
                Some("à esquerda") => BinOp::Shl,
                Some("à direita") => BinOp::Shr,
                _ => return err(Diagnostic::new(L0216, "O deslocamento é \"à esquerda\" ou \"à direita\".", dir.span)),
            };
            let span = at.span.to(self.prev_span());
            self.last_nominal = true;
            return Ok(Some(Expr { kind: ExprKind::Binary { op, lhs: Box::new(value), rhs: Box::new(amount) }, span }));
        }
        if let NominalOp::Binary(op) = nominal.op {
            check_gender(nominal.gender)?;
            let prep = self.bump();
            if prep.is_word("entre") && !nominal.commutative {
                return err(Diagnostic::new(
                    L0217,
                    format!("\"entre\" indica simetria e só vale para operação comutativa; \"{w}\" pede \"de\"."),
                    prep.span,
                ));
            }
            let lhs = self.nominal_operand()?;
            if !self.eat_word("e") {
                return Err(self.expected(&format!("\"e\" e o segundo termo de \"{w}\"")));
            }
            // "a diferença do x de p e do x de q": o segundo termo pode repetir o "de".
            if self.at_word("de") && self.peek().contraction.is_some() {
                self.bump();
            }
            let rhs = self.nominal_operand()?;
            let span = at.span.to(self.prev_span());
            self.last_nominal = true;
            return Ok(Some(Expr { kind: ExprKind::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }, span }));
        }
        let NominalOp::Unary(op) = nominal.op else { unreachable!("deslocamento e binárias tratados acima") };
        check_gender(nominal.gender)?;
        self.expect_word("de")?;
        let operand = self.nominal_operand()?;
        let span = at.span.to(self.prev_span());
        self.last_nominal = true;
        Ok(Some(Expr { kind: ExprKind::Unary { op, operand: Box::new(operand) }, span }))
    }

    /// Termo de operação nominal: um valor simples, sem operação infixa. Para misturar,
    /// parênteses.
    fn nominal_operand(&mut self) -> Result<Expr> {
        let e = self.primary()?;
        if self.at_word("como") {
            let t = self.peek().clone();
            return err(Diagnostic::new(L0133, "Conversão encostada numa operação por extenso é ambígua: precisa de parênteses.", t.span)
                .hint("Escreva \"(a soma de 3 e 4) como real\" ou \"a soma de 3 e (4 como real)\", conforme o que quer dizer."));
        }
        if self.at_infix() {
            let t = self.peek().clone();
            return err(Diagnostic::new(L0133, "Dentro de uma operação por extenso, a conta infixa precisa de parênteses.", t.span)
                .hint("Escreva \"a diferença de 10 e (3 mais 1)\" ou \"(a diferença de 10 e 3) mais 1\", conforme o que quer dizer."));
        }
        self.last_nominal = false;
        Ok(e)
    }

    /// O próximo token é um operador infixo?
    pub(super) fn at_infix(&self) -> bool {
        self.at_word("mais")
            || self.at_word("menos")
            || self.at_word("dividido por")
            || self.at_word("módulo")
            || self.at_word("como")
            || (self.at_word("vezes") && self.starts_value(1))
    }

    /// "um círculo", "uma estrela": caso de enumeração, com o gênero conferido.
    pub(super) fn variant_pattern(&mut self) -> Result<Pattern> {
        let at = self.bump();
        let wt = self.bump();
        let w = wt.word().unwrap_or_default().to_string();
        let Some(&(e, v)) = self.vocab.variants.get(&w) else {
            return err(Diagnostic::new(L0218, format!("\"{}\" não é caso de nenhuma enumeração.", wt.orig), wt.span));
        };
        let var = &self.prog.enums[e].variants[v];
        let a = at.word().and_then(morphology::article).filter(|a| !a.definite);
        if a.is_none_or(|a| a.gender != var.gender || a.number != Number::Singular) {
            let right = morphology::indefinite_article(var.gender, Number::Singular);
            return err(Diagnostic::new(L0219, format!("O caso se diz com artigo indefinido: \"{right} {w}\"."), at.span.to(wt.span)).fix(
                format!("Escrever \"{right} {w}\""),
                at.span.to(wt.span),
                format!("{right} {w}"),
            ));
        }
        Ok(Pattern::Variant { word: w, gender: var.gender, span: at.span.to(wt.span) })
    }

    /// "uma conta com o saldo 100 e o titular \"Ana\"".
    pub(super) fn construct(&mut self) -> Result<Option<Expr>> {
        let at = self.peek().clone();
        let Some(art) = at.word().and_then(morphology::article).filter(|a| !a.definite) else { return Ok(None) };
        let wt = self.peek_at(1).clone();
        let Some(w) = wt.word() else { return Ok(None) };
        let (word, gender) = if let Some(&id) = self.vocab.records.get(w) {
            let r = &self.prog.records[id];
            (r.word.clone(), r.gender)
        } else if let Some(&(e, v)) = self.vocab.variants.get(w) {
            let v = &self.prog.enums[e].variants[v];
            (v.word.clone(), v.gender)
        } else {
            return Ok(None);
        };
        if gender != art.gender || art.number != Number::Singular {
            let right = morphology::indefinite_article(gender, Number::Singular);
            return err(Diagnostic::new(L0220, format!("\"{w}\" é {}: \"{right} {w}\".", gender.describe()), at.span.to(wt.span)).fix(
                format!("Escrever \"{right} {w}\""),
                at.span.to(wt.span),
                format!("{right} {w}"),
            ));
        }
        self.bump();
        self.bump();
        let mut fields = Vec::new();
        if self.eat_word("com") {
            loop {
                let n = self.name_ref()?;
                let value = self.expr()?;
                fields.push((n.word.clone(), n.span, value));
                if (self.at_word("e") || self.at_punct(','))
                    && ((self.definite_article_at(1).is_some()
                        && self.peek_at(2).word().is_some_and(|w| self.vocab.field_names.contains(w)))
                        || self.peek_at(1).word().is_some_and(|w| morphology::is_symbol(w) && self.vocab.field_names.contains(w)))
                {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let span = at.span.to(self.prev_span());
        Ok(Some(Expr { kind: ExprKind::Construct { word, gender, word_span: wt.span, fields }, span }))
    }
}

enum Segment {
    Name(NameRef),
    Index(Expr, Span),
    Ordinal { position: u32, gender: Gender, span: Span },
}
