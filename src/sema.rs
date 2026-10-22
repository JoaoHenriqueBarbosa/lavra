//! Análise semântica: resolve nomes e escopos, infere e confere tipos, confere
//! mutabilidade, e baixa a AST para a HIR.

use crate::ast::{self, BinOp, ClauseKind, ExprKind, Init};
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
        let mut cx = FnCx { locals: Vec::new(), scopes: vec![Vec::new()], warnings: Vec::new() };
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
            match s {
                ast::Stmt::Period(p) => {
                    for c in &p.clauses {
                        self.clause(c, &mut out)?;
                    }
                }
                _ => return err(Diagnostic::new(Category::Syntax, "Construção ainda não suportada.", Span::new(0, 0))),
            }
        }
        self.scopes.pop();
        Ok(hir::Block(out))
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
                if let Some(id) = self.scopes.last().unwrap().iter().find(|(w, _)| *w == name.word).map(|(_, id)| *id) {
                    return err(Diagnostic::new(Category::Scope, format!("\"{}\" já foi declarado neste bloco.", name.display()), name.span)
                        .label(self.locals[id].decl_span, "declarado aqui"));
                }
                let id = self.locals.len();
                self.locals.push(LocalInfo {
                    local: hir::Local { name: name.word.clone(), ty, mutable: *mutable, span: name.span },
                    gender: name.gender(),
                    number: name.number(),
                    mutated: false,
                    decl_span: name.span,
                });
                self.scopes.last_mut().unwrap().push((name.word.clone(), id));
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
