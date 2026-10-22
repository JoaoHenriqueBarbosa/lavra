//! Análise semântica: resolve nomes e escopos, infere e confere tipos, confere
//! mutabilidade, e baixa a AST para a HIR.

use crate::ast::{self, BinOp, ClauseKind, ExprKind, Init, Rel};
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::hir;
use crate::morphology::{self, Gender, Number};
use crate::span::Span;
use crate::types::{IntTy, Ty};

pub fn check(prog: &ast::Program) -> Result<(hir::Program, Vec<Diagnostic>)> {
    let Some(main) = prog.main else {
        return err(Diagnostic::new(Category::Syntax, "O programa não tem \"Para começar:\".", Span::new(0, 0))
            .hint("Todo programa começa por \"Para começar:\", seguido do bloco recuado."));
    };
    let mut funcs = Vec::new();
    let mut warnings = Vec::new();
    let mut main_index = 0;
    for f in &prog.funcs {
        let sig = &prog.sigs[f.sig];
        let mut cx = FnCx { locals: Vec::new(), scopes: vec![Vec::new()], warnings: Vec::new(), loops: 0, result: None };
        let body = cx.block(&f.body)?;
        cx.finish();
        warnings.append(&mut cx.warnings);
        if f.sig == main {
            main_index = funcs.len();
        }
        funcs.push(hir::Func { name: sig.infinitive.clone(), locals: cx.locals.into_iter().map(|l| l.local).collect(), body, ret: Ty::Unit });
    }
    Ok((hir::Program { funcs, main: main_index }, warnings))
}

struct LocalInfo {
    local: hir::Local,
    gender: Gender,
    number: Number,
    /// Já mudou alguma vez (para o aviso de "começa" que nunca muda).
    mutated: bool,
    decl_span: Span,
}

struct FnCx {
    locals: Vec<LocalInfo>,
    scopes: Vec<Vec<(String, hir::LocalId)>>,
    warnings: Vec<Diagnostic>,
    /// Laços abertos (para "Paro" e "Pulo").
    loops: usize,
    /// Local escondido com o valor da última oração do período ("o resultado").
    result: Option<hir::LocalId>,
}

impl FnCx {
    fn finish(&mut self) {
        for l in &self.locals {
            if l.local.mutable && !l.mutated {
                let name = format!("{} {}", morphology::definite_article(l.gender, l.number), l.local.name);
                let verb = morphology::third_person("ser", l.number).unwrap_or_default();
                self.warnings.push(
                    Diagnostic::new(Category::Warning, format!("{} começa com um valor e nunca muda.", capitalize(&name)), l.decl_span)
                        .hint(format!("Se é definição, escreva \"{} {verb} ...\".", capitalize(&name))),
                );
            }
        }
    }

    fn lookup(&self, word: &str) -> Option<hir::LocalId> {
        self.scopes.iter().rev().find_map(|s| s.iter().rev().find(|(w, _)| w == word).map(|(_, id)| *id))
    }

    fn block(&mut self, b: &ast::Block) -> Result<hir::Block> {
        self.scopes.push(Vec::new());
        let mut out = Vec::new();
        for s in &b.stmts {
            self.stmt(s, &mut out)?;
        }
        self.scopes.pop();
        Ok(hir::Block(out))
    }

    fn stmt(&mut self, s: &ast::Stmt, out: &mut Vec<hir::Stmt>) -> Result<()> {
        match s {
            ast::Stmt::Period(p) => {
                self.result = None;
                for c in &p.clauses {
                    self.clause(c, out)?;
                }
                self.result = None;
            }
            ast::Stmt::If { arms, otherwise, .. } => {
                let mut harms = Vec::new();
                for (c, b) in arms {
                    harms.push((self.cond(c)?, self.block(b)?));
                }
                let otherwise = otherwise.as_ref().map(|b| self.block(b)).transpose()?;
                out.push(hir::Stmt::If { arms: harms, otherwise });
            }
            ast::Stmt::While { cond, body, .. } => {
                let cond = self.cond(cond)?;
                let body = self.loop_body(body)?;
                out.push(hir::Stmt::While { cond, body });
            }
            ast::Stmt::Repeat { count, body, .. } => {
                let count = match count {
                    Some(c) => {
                        let e = self.expr(c, None)?;
                        if !matches!(e.ty, Ty::Int(_) | Ty::Error) {
                            return err(Diagnostic::new(Category::Type, format!("Repete-se um número inteiro de vezes, e este valor é {}.", e.ty.describe()), c.span));
                        }
                        Some(e)
                    }
                    None => None,
                };
                let body = self.loop_body(body)?;
                out.push(hir::Stmt::Repeat { count, body });
            }
            ast::Stmt::ForRange { var, from, to, body, .. } => {
                let from = self.expr(from, None)?;
                if !matches!(from.ty, Ty::Int(_)) {
                    return err(Diagnostic::new(Category::Type, format!("O intervalo de \"Para cada\" é de inteiros, e este valor é {}.", from.ty.describe()), from.span));
                }
                let to = self.expr(to, Some(&from.ty))?;
                self.scopes.push(Vec::new());
                let id = self.declare(var, from.ty.clone(), false)?;
                let body = self.loop_body(body)?;
                self.scopes.pop();
                out.push(hir::Stmt::ForRange { var: id, from, to, body });
            }
            _ => return err(Diagnostic::new(Category::Syntax, "Construção ainda não suportada.", Span::new(0, 0))),
        }
        Ok(())
    }

    fn loop_body(&mut self, b: &ast::Block) -> Result<hir::Block> {
        self.loops += 1;
        let r = self.block(b);
        self.loops -= 1;
        r
    }

    fn declare(&mut self, name: &ast::NameRef, ty: Ty, mutable: bool) -> Result<hir::LocalId> {
        if let Some(id) = self.scopes.last().unwrap().iter().find(|(w, _)| *w == name.word).map(|(_, id)| *id) {
            return err(Diagnostic::new(Category::Scope, format!("\"{}\" já foi declarado neste bloco.", name.display()), name.span)
                .label(self.locals[id].decl_span, "declarado aqui"));
        }
        let id = self.locals.len();
        self.locals.push(LocalInfo {
            local: hir::Local { name: name.word.clone(), ty, mutable, span: name.span },
            gender: name.gender(),
            number: name.number(),
            mutated: false,
            decl_span: name.span,
        });
        self.scopes.last_mut().unwrap().push((name.word.clone(), id));
        Ok(id)
    }

    /// Guarda o valor num local escondido que "o resultado" retoma.
    fn set_result(&mut self, value: hir::Expr, out: &mut Vec<hir::Stmt>) {
        let id = self.locals.len();
        self.locals.push(LocalInfo {
            local: hir::Local { name: "resultado".into(), ty: value.ty.clone(), mutable: false, span: value.span },
            gender: Gender::Masculine,
            number: Number::Singular,
            mutated: true,
            decl_span: value.span,
        });
        out.push(hir::Stmt::Let { local: id, value });
        self.result = Some(id);
    }

    fn cond(&mut self, c: &ast::Cond) -> Result<hir::Cond> {
        Ok(match c {
            ast::Cond::And(v) => hir::Cond::And(v.iter().map(|c| self.cond(c)).collect::<Result<_>>()?),
            ast::Cond::Or(v) => hir::Cond::Or(v.iter().map(|c| self.cond(c)).collect::<Result<_>>()?),
            ast::Cond::Atom { negated, kind, span } => {
                let inner = match kind {
                    ast::CondKind::Rel { subject, rel, rhs } => {
                        let lhs = self.expr(subject, None)?;
                        let rhs = self.expr(rhs, Some(&lhs.ty))?;
                        let ordered = !matches!(rel, Rel::Equal | Rel::Different);
                        if ordered && !matches!(lhs.ty, Ty::Int(_) | Ty::Real(_) | Ty::Char | Ty::Text | Ty::Error) {
                            return err(Diagnostic::new(Category::Type, format!("\"{}\" compara números, caracteres ou textos, e este valor é {}.", rel.words(), lhs.ty.describe()), *span));
                        }
                        hir::Cond::Cmp { rel: *rel, lhs, rhs }
                    }
                    _ => return err(Diagnostic::new(Category::Syntax, "Condição ainda não suportada.", *span)),
                };
                if *negated { hir::Cond::Not(Box::new(inner)) } else { inner }
            }
        })
    }

    fn clause(&mut self, c: &ast::Clause, out: &mut Vec<hir::Stmt>) -> Result<()> {
        match &c.kind {
            ClauseKind::Decl { name, ann, mutable, init } => {
                if name.is_symbol() && name.word.len() > 1 {
                    return err(Diagnostic::new(Category::Syntax, "Nome inválido.", name.span));
                }
                let Init::Expr(e) = init else { unreachable!() };
                let annotated = ann.as_ref().map(|a| self.type_of(a)).transpose()?;
                let value = self.expr(e, annotated.as_ref())?;
                let ty = annotated.unwrap_or_else(|| value.ty.clone());
                let id = self.declare(name, ty, *mutable)?;
                out.push(hir::Stmt::Let { local: id, value });
            }
            ClauseKind::Assign { place, value } => {
                let p = self.place_mut(place, "vira")?;
                let Init::Expr(e) = value else { unreachable!() };
                let v = self.expr(e, Some(&p.ty))?;
                out.push(hir::Stmt::Assign { place: p, value: v });
            }
            ClauseKind::Grow { place, value } | ClauseKind::Shrink { place, value } => {
                let grow = matches!(c.kind, ClauseKind::Grow { .. });
                let p = self.place_mut(place, if grow { "cresce" } else { "diminui" })?;
                if !p.ty.is_numeric() {
                    return err(Diagnostic::new(Category::Type, format!("Só número cresce ou diminui; {} é {}.", place_name(place), p.ty.describe()), place.span));
                }
                let v = self.expr(value, Some(&p.ty))?;
                let op = if grow { BinOp::Add } else { BinOp::Sub };
                out.push(hir::Stmt::Compound { place: p, op, value: v, span: c.span });
            }
            ClauseKind::Write { parts, newline } => {
                let value = if parts.len() == 1 { self.expr(&parts[0], None)? } else { unreachable!() };
                out.push(hir::Stmt::Write { value, newline: *newline });
                self.result = None;
            }
            ClauseKind::ReadInto { place } => {
                let p = self.place_mut(place, "muda pela leitura")?;
                if !readable(&p.ty) {
                    return err(Diagnostic::new(Category::Type, format!("Não se lê {} da entrada.", p.ty.describe()), place.span));
                }
                let value = hir::Expr { kind: hir::ExprKind::Input, ty: p.ty.clone(), span: c.span };
                out.push(hir::Stmt::Assign { place: p.clone(), value });
                let span = p.span;
                let ty = p.ty.clone();
                self.set_result(hir::Expr { kind: hir::ExprKind::Read(p), ty, span }, out);
            }
            ClauseKind::ReadValue { ty } => {
                let ty = self.type_of(ty)?;
                self.set_result(hir::Expr { kind: hir::ExprKind::Input, ty, span: c.span }, out);
            }
            ClauseKind::Break | ClauseKind::Continue => {
                let brk = matches!(c.kind, ClauseKind::Break);
                if self.loops == 0 {
                    let w = if brk { "Paro" } else { "Pulo" };
                    return err(Diagnostic::new(Category::Syntax, format!("\"{w}\" só faz sentido dentro de um laço."), c.span));
                }
                out.push(if brk { hir::Stmt::Break } else { hir::Stmt::Continue });
            }
            _ => return err(Diagnostic::new(Category::Syntax, "Oração ainda não suportada.", c.span)),
        }
        Ok(())
    }

    fn type_of(&self, a: &ast::TypeAnn) -> Result<Ty> {
        Ok(match &a.ty {
            ast::TypeExpr::Int { bits, signed } => Ty::Int(IntTy { bits: *bits, signed: *signed }),
            ast::TypeExpr::Byte => Ty::Int(IntTy { bits: 8, signed: false }),
            ast::TypeExpr::Real { bits } => Ty::Real(*bits),
            ast::TypeExpr::Bool => Ty::Bool,
            ast::TypeExpr::Char => Ty::Char,
            ast::TypeExpr::Text => Ty::Text,
            _ => return err(Diagnostic::new(Category::Type, "Tipo ainda não suportado.", a.span)),
        })
    }

    fn resolve(&self, name: &ast::NameRef) -> Result<hir::LocalId> {
        let Some(id) = self.lookup(&name.word) else {
            let mut d = Diagnostic::new(Category::Scope, format!("{} não foi declarado.", capitalize(&format!("\"{}\"", name.display()))), name.span);
            let near = self
                .scopes
                .iter()
                .flatten()
                .map(|(w, _)| w)
                .filter(|w| morphology::edit_distance(w, &name.word) <= 2)
                .min_by_key(|w| morphology::edit_distance(w, &name.word));
            if let Some(w) = near {
                d = d.hint(format!("Quis dizer \"{w}\"?"));
            } else {
                d = d.hint(format!("Declare antes de usar: \"{} é ...\" ou \"{} começa em ...\".", capitalize(&name.display()), capitalize(&name.display())));
            }
            return err(d);
        };
        let l = &self.locals[id];
        if let Some(a) = &name.article
            && (a.gender != l.gender || a.number != l.number)
        {
            let right = morphology::definite_article(l.gender, l.number);
            let written = match a.contraction {
                Some(c) => format!("{c} {}", name.word),
                None => name.display(),
            };
            let fixed = match a.contraction.and_then(morphology::split_contraction) {
                Some((_, prep, _)) => format!("{} {}", morphology::join_prep(prep, right), name.word),
                None => format!("{right} {}", name.word),
            };
            return err(Diagnostic::new(
                Category::Agreement,
                format!("\"{written}\" não concorda: \"{}\" é {}. Escreva \"{fixed}\".", name.word, morphology::describe_gn(l.gender, l.number)),
                name.span,
            )
            .label(l.decl_span, "declarado aqui"));
        }
        Ok(id)
    }

    fn place_read(&mut self, p: &ast::Place) -> Result<hir::Place> {
        let root = self.resolve(&p.root)?;
        Ok(hir::Place { root, ty: self.locals[root].local.ty.clone(), span: p.span })
    }

    fn place_mut(&mut self, p: &ast::Place, verb: &str) -> Result<hir::Place> {
        let hp = self.place_read(p)?;
        let l = &mut self.locals[hp.root];
        if !l.local.mutable {
            let name = place_name(p);
            let be = morphology::third_person("ser", l.number).unwrap_or_default();
            let begin = morphology::third_person("começar", l.number).unwrap_or_default();
            let fixed = morphology::inflect_adjective("fixo", Gender::Masculine, l.gender, l.number);
            return err(Diagnostic::new(Category::Mutability, format!("{} {be} {fixed} por definição e não {verb}.", capitalize(&name)), p.span)
                .label(l.decl_span, format!("definido com \"{be}\""))
                .hint(format!("Quem varia se declara com \"{begin} em\": \"{} {begin} em ...\".", capitalize(&name))));
        }
        l.mutated = true;
        Ok(hp)
    }

    fn expr(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Result<hir::Expr> {
        let (kind, ty) = match &e.kind {
            ExprKind::Int(v) => {
                let ty = match expected {
                    Some(Ty::Int(i)) => Ty::Int(*i),
                    Some(Ty::Real(b)) => return Ok(hir::Expr { kind: hir::ExprKind::Real(*v as f64), ty: Ty::Real(*b), span: e.span }),
                    _ => Ty::Int(IntTy::I64),
                };
                if let Ty::Int(i) = ty
                    && !i.contains(*v)
                {
                    return err(Diagnostic::new(Category::Type, format!("{v} não cabe num {}: vai de {} a {}.", i.noun(false), i.min(), i.max()), e.span));
                }
                (hir::ExprKind::Int(*v), ty)
            }
            ExprKind::Real(v) => (hir::ExprKind::Real(*v), Ty::Real(match expected {
                Some(Ty::Real(b)) => *b,
                _ => 64,
            })),
            ExprKind::Text(s) => (hir::ExprKind::Text(s.clone()), Ty::Text),
            ExprKind::Char(c) => (hir::ExprKind::Char(*c), Ty::Char),
            ExprKind::Bool(b) => (hir::ExprKind::Bool(*b), Ty::Bool),
            ExprKind::Place(p) => {
                let hp = self.place_read(p)?;
                let ty = hp.ty.clone();
                (hir::ExprKind::Read(hp), ty)
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let num_hint = expected.filter(|t| t.is_numeric());
                let l = self.expr(lhs, num_hint)?;
                let r = self.expr(rhs, Some(&l.ty))?;
                let ty = self.binary_type(*op, &l, &r, e.span)?;
                (hir::ExprKind::Binary(*op, Box::new(l), Box::new(r)), ty)
            }
            ExprKind::Anaphora => {
                let Some(id) = self.result else {
                    return err(Diagnostic::new(Category::Scope, "\"O resultado\" retoma o valor da oração anterior do mesmo período, e aqui nenhuma oração antes deu valor.", e.span)
                        .hint("Junte as orações no mesmo período: \"Leio um inteiro e escrevo o resultado.\""));
                };
                let ty = self.locals[id].local.ty.clone();
                (hir::ExprKind::Read(hir::Place { root: id, ty: ty.clone(), span: e.span }), ty)
            }
            ExprKind::Concat(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    out.push(self.expr(p, None)?);
                }
                (hir::ExprKind::Concat(out), Ty::Text)
            }
            _ => return err(Diagnostic::new(Category::Syntax, "Expressão ainda não suportada.", e.span)),
        };
        let out = hir::Expr { kind, ty, span: e.span };
        if let Some(exp) = expected
            && !compatible(exp, &out.ty)
        {
            return err(Diagnostic::new(Category::Type, format!("Esperava {}, mas este valor é {}.", exp.describe(), out.ty.describe()), e.span));
        }
        Ok(out)
    }

    fn binary_type(&self, op: BinOp, l: &hir::Expr, r: &hir::Expr, span: Span) -> Result<Ty> {
        if op == BinOp::Add && (l.ty == Ty::Text || r.ty == Ty::Text) {
            return err(Diagnostic::new(Category::Type, "Texto não se soma: se junta.", span)
                .hint("Para juntar textos, use \"seguido de\": Escrevo \"total: \" seguido do total."));
        }
        if !l.ty.is_numeric() || !r.ty.is_numeric() {
            let bad = if l.ty.is_numeric() { r } else { l };
            return err(Diagnostic::new(Category::Type, format!("\"{}\" só opera números, e este valor é {}.", op.words(), bad.ty.describe()), bad.span));
        }
        if l.ty != r.ty && !l.ty.is_error() && !r.ty.is_error() {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" entre {} e {}: os dois lados precisam ter o mesmo tipo.", op.words(), l.ty.describe(), r.ty.describe()), span)
                .hint("Converta um dos lados com \"como\": \"o total como real\"."));
        }
        Ok(l.ty.clone())
    }
}

fn readable(t: &Ty) -> bool {
    matches!(t, Ty::Int(_) | Ty::Real(_) | Ty::Bool | Ty::Char | Ty::Text | Ty::Error)
}

fn compatible(expected: &Ty, got: &Ty) -> bool {
    expected == got || expected.is_error() || got.is_error()
}

fn place_name(p: &ast::Place) -> String {
    p.root.display()
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
