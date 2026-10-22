//! Análise semântica: resolve nomes e escopos, infere e confere tipos, confere
//! mutabilidade e regência dos parâmetros, e baixa a AST para a HIR.

use crate::ast::{self, BinOp, ClauseKind, ExprKind, Init, ParamMode, Rel, UnOp};
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::hir::{self, Var};
use crate::morphology::{self, Gender, Number};
use crate::span::Span;
use crate::types::{IntTy, RecordTy, Ty};
use std::collections::HashMap;
use std::sync::Arc;

pub fn check(prog: &ast::Program) -> Result<(hir::Program, Vec<Diagnostic>)> {
    let Some(main) = prog.main else {
        return err(Diagnostic::new(Category::Syntax, "O programa não tem \"Para começar:\".", Span::new(0, 0))
            .hint("Todo programa começa por \"Para começar:\", seguido do bloco recuado."));
    };
    let mut env = Env {
        prog,
        globals: Vec::new(),
        sig_types: Vec::new(),
        sig_func: vec![usize::MAX; prog.sigs.len()],
        warnings: Vec::new(),
        records: HashMap::new(),
    };
    for (i, f) in prog.funcs.iter().enumerate() {
        env.sig_func[f.sig] = i;
    }
    for i in 0..prog.records.len() {
        env.build_record(i, &mut Vec::new())?;
    }
    for sig in &prog.sigs {
        let params = sig.params.iter().map(|p| env.type_of(&p.ty)).collect::<Result<Vec<_>>>()?;
        let ret = sig.ret.as_ref().map(|t| env.type_of(t)).transpose()?.unwrap_or(Ty::Unit);
        env.sig_types.push(SigTypes { params, ret });
    }

    // Nível de fora: só constantes.
    let mut init = FnCx::new(&mut env, None, Span::default());
    init.top = true;
    let mut init_body = Vec::new();
    for s in &prog.top {
        init.stmt(s, &mut init_body)?;
    }
    let init = init.into_func("início".into(), Vec::new(), hir::Block(init_body), Span::default())?;

    let mut funcs = Vec::new();
    for f in &prog.funcs {
        let sig = &prog.sigs[f.sig];
        let ret = env.sig_types[f.sig].ret.clone();
        let mut cx = FnCx::new(&mut env, Some(f.sig), sig.span);
        let mut params = Vec::new();
        for (p, ty) in sig.params.iter().zip(env_param_types(&cx, f.sig)) {
            let id = cx.declare(&p.name, ty, p.mode == ParamMode::Lent)?;
            cx.locals[id].mutated = true;
            params.push((id, p.mode));
        }
        let body = cx.block(&f.body)?;
        if ret != Ty::Unit && !always_returns(&body) {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" promete devolver {}, mas pode terminar sem devolver.", sig.infinitive, ret.describe()), sig.name_span)
                .hint("Garanta um \"Devolvo ...\" em todo caminho até o fim da função."));
        }
        funcs.push(cx.into_func(sig.infinitive.clone(), params, body, sig.span)?);
    }
    let warnings = std::mem::take(&mut env.warnings);
    let globals = env
        .globals
        .into_iter()
        .map(|g| hir::Global { display: format!("{} {}", morphology::definite_article(g.gender, g.number), g.name), name: g.name, ty: g.ty })
        .collect();
    Ok((hir::Program { funcs, globals, init, main: env.sig_func[main] }, warnings))
}

fn env_param_types(cx: &FnCx, sig: usize) -> Vec<Ty> {
    cx.env.sig_types[sig].params.clone()
}

struct SigTypes {
    params: Vec<Ty>,
    ret: Ty,
}

struct GlobalInfo {
    name: String,
    ty: Ty,
    gender: Gender,
    number: Number,
    span: Span,
}

struct Env<'a> {
    prog: &'a ast::Program,
    globals: Vec<GlobalInfo>,
    sig_types: Vec<SigTypes>,
    sig_func: Vec<hir::FuncId>,
    warnings: Vec<Diagnostic>,
    records: HashMap<String, Arc<RecordTy>>,
}

impl Env<'_> {
    /// Monta o tipo do registro `i` depois dos registros de que ele depende. Um
    /// registro que contém a si mesmo teria tamanho infinito.
    fn build_record(&mut self, i: usize, stack: &mut Vec<usize>) -> Result<Arc<RecordTy>> {
        let def = &self.prog.records[i];
        if let Some(r) = self.records.get(&def.word) {
            return Ok(r.clone());
        }
        if stack.contains(&i) {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" contém a si mesmo e teria tamanho infinito.", def.word), def.span)
                .hint("Para guardar outros do mesmo tipo, use uma lista: \"os filhos, uma lista de nós\"."));
        }
        stack.push(i);
        let mut fields = Vec::new();
        for f in &def.fields {
            fields.push((f.name.word.clone(), self.type_in_record(&f.ty.ty, f.ty.span, stack)?));
        }
        stack.pop();
        let field_articles = def.fields.iter().map(|f| (f.name.word.clone(), morphology::definite_article(f.name.gender(), f.name.number()))).collect();
        let r = Arc::new(RecordTy { id: i, name: def.word.clone(), plural: def.plural.clone(), feminine: def.gender == Gender::Feminine, fields, field_articles });
        self.records.insert(def.word.clone(), r.clone());
        Ok(r)
    }

    fn type_in_record(&mut self, t: &ast::TypeExpr, span: Span, stack: &mut Vec<usize>) -> Result<Ty> {
        Ok(match t {
            ast::TypeExpr::Named(name) => {
                let i = self.prog.records.iter().position(|r| &r.word == name).unwrap();
                Ty::Record(self.build_record(i, stack)?)
            }
            // Dentro de uma lista o registro pode citar a si mesmo: a lista guarda por fora.
            ast::TypeExpr::List(inner) => {
                if let ast::TypeExpr::Named(name) = inner.as_ref()
                    && let Some(i) = self.prog.records.iter().position(|r| &r.word == name)
                    && stack.contains(&i)
                {
                    return err(Diagnostic::new(Category::Type, "Registro recursivo ainda não é suportado.", span));
                }
                Ty::List(Box::new(self.type_in_record(inner, span, stack)?))
            }
            ast::TypeExpr::Maybe(inner) => Ty::Maybe(Box::new(self.type_in_record(inner, span, stack)?)),
            other => simple_type(other).ok_or_else(|| Box::new(Diagnostic::new(Category::Type, "Tipo ainda não suportado.", span)))?,
        })
    }

    fn type_of(&self, a: &ast::TypeAnn) -> Result<Ty> {
        self.type_expr(&a.ty, a.span)
    }

    fn type_expr(&self, t: &ast::TypeExpr, span: Span) -> Result<Ty> {
        Ok(match t {
            ast::TypeExpr::Named(name) => Ty::Record(self.records[name].clone()),
            ast::TypeExpr::List(inner) => Ty::List(Box::new(self.type_expr(inner, span)?)),
            ast::TypeExpr::Maybe(inner) => Ty::Maybe(Box::new(self.type_expr(inner, span)?)),
            other => simple_type(other).ok_or_else(|| Box::new(Diagnostic::new(Category::Type, "Tipo ainda não suportado.", span)))?,
        })
    }
}

fn simple_type(t: &ast::TypeExpr) -> Option<Ty> {
    Some(match t {
        ast::TypeExpr::Int { bits, signed } => Ty::Int(IntTy { bits: *bits, signed: *signed }),
        ast::TypeExpr::Byte => Ty::Int(IntTy { bits: 8, signed: false }),
        ast::TypeExpr::Real { bits } => Ty::Real(*bits),
        ast::TypeExpr::Bool => Ty::Bool,
        ast::TypeExpr::Char => Ty::Char,
        ast::TypeExpr::Text => Ty::Text,
        _ => return None,
    })
}

struct LocalInfo {
    local: hir::Local,
    gender: Gender,
    number: Number,
    /// Já mudou alguma vez (para o aviso de "começa" que nunca muda).
    mutated: bool,
    decl_span: Span,
    hidden: bool,
}

struct FnCx<'e, 'a> {
    env: &'e mut Env<'a>,
    sig: Option<usize>,
    locals: Vec<LocalInfo>,
    scopes: Vec<Vec<(String, hir::LocalId)>>,
    /// Laços abertos (para "Paro" e "Pulo").
    loops: usize,
    /// Local escondido com o valor da última oração do período ("o resultado").
    result: Option<hir::LocalId>,
    /// Corpo do nível de fora: as declarações viram constantes globais.
    top: bool,
    /// Variáveis de tipo da inferência: `Ty::Infer(i)` vale `subst[i]` quando resolvida.
    subst: Vec<(Option<Ty>, Span)>,
    /// A próxima chamada já tem o que fazer se falhar (", ou desisto", ", ou VALOR").
    handled: bool,
}

/// O que se sabe de um nome resolvido.
struct NameInfo {
    var: Var,
    ty: Ty,
    mutable: bool,
    gender: Gender,
    number: Number,
    decl_span: Span,
}

impl<'e, 'a> FnCx<'e, 'a> {
    fn new(env: &'e mut Env<'a>, sig: Option<usize>, _span: Span) -> Self {
        FnCx { env, sig, locals: Vec::new(), scopes: vec![Vec::new()], loops: 0, result: None, top: false, subst: Vec::new(), handled: false }
    }

    fn into_func(mut self, name: String, params: Vec<(hir::LocalId, ParamMode)>, mut body: hir::Block, span: Span) -> Result<hir::Func> {
        for i in 0..self.locals.len() {
            let t = self.resolve_ty(&self.locals[i].local.ty);
            if contains_infer(&t) {
                let l = &self.locals[i];
                let name = format!("{} {}", morphology::definite_article(l.gender, l.number), l.local.name);
                return err(Diagnostic::new(Category::Type, format!("Não sei o tipo dos itens de \"{name}\": nada entra nem sai dela."), l.decl_span)
                    .hint(format!("Diga no aposto: \"{}, uma lista de inteiros, ...\".", capitalize(&name))));
            }
            self.locals[i].local.ty = t;
        }
        let subst = std::mem::take(&mut self.subst);
        resolve_block(&mut body, &subst);
        for l in &self.locals {
            if l.local.mutable && !l.mutated && !l.hidden {
                let name = format!("{} {}", morphology::definite_article(l.gender, l.number), l.local.name);
                let verb = morphology::third_person("ser", l.number).unwrap_or_default();
                let begin = morphology::third_person("começar", l.number).unwrap_or_default();
                self.env.warnings.push(
                    Diagnostic::new(Category::Warning, format!("{} {begin} com um valor e nunca muda.", capitalize(&name)), l.decl_span)
                        .hint(format!("Se é definição, escreva \"{} {verb} ...\".", capitalize(&name))),
                );
            }
        }
        let ret = match self.sig {
            Some(s) => self.env.sig_types[s].ret.clone(),
            None => Ty::Unit,
        };
        let fallible = self.sig.is_some_and(|s| self.env.prog.sigs[s].fallible);
        Ok(hir::Func { name, params, fallible, locals: self.locals.into_iter().map(|l| l.local).collect(), body, ret, span })
    }

    fn fresh(&mut self, span: Span) -> Ty {
        self.subst.push((None, span));
        Ty::Infer(self.subst.len() as u32 - 1)
    }

    fn resolve_ty(&self, t: &Ty) -> Ty {
        resolve(t, &self.subst)
    }

    /// Unifica o tipo esperado com o obtido, resolvendo variáveis de tipo.
    fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let a = self.resolve_ty(a);
        let b = self.resolve_ty(b);
        match (&a, &b) {
            (Ty::Infer(i), Ty::Infer(j)) if i == j => true,
            (Ty::Infer(i), t) | (t, Ty::Infer(i)) => {
                self.subst[*i as usize].0 = Some(t.clone());
                true
            }
            (Ty::List(x), Ty::List(y)) | (Ty::Maybe(x), Ty::Maybe(y)) => self.unify(x, y),
            (Ty::Error, _) | (_, Ty::Error) => true,
            _ => a == b,
        }
    }

    fn lookup(&self, word: &str) -> Option<NameInfo> {
        if let Some(id) = self.scopes.iter().rev().find_map(|s| s.iter().rev().find(|(w, _)| w == word).map(|(_, id)| *id)) {
            let l = &self.locals[id];
            return Some(NameInfo { var: Var::Local(id), ty: l.local.ty.clone(), mutable: l.local.mutable, gender: l.gender, number: l.number, decl_span: l.decl_span });
        }
        let (id, g) = self.env.globals.iter().enumerate().find(|(_, g)| g.name == word)?;
        Some(NameInfo { var: Var::Global(id), ty: g.ty.clone(), mutable: false, gender: g.gender, number: g.number, decl_span: g.span })
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
        if self.top && !matches!(s, ast::Stmt::Period(_)) {
            return Err(top_level_error(stmt_span(s)));
        }
        match s {
            ast::Stmt::Period(p) => {
                self.result = None;
                for c in &p.clauses {
                    if self.top && !matches!(c.kind, ClauseKind::Decl { mutable: false, .. }) {
                        return Err(top_level_error(c.span));
                    }
                    self.clause(c, out)?;
                }
                self.result = None;
            }
            ast::Stmt::If { arms, otherwise, .. } => {
                let mut harms = Vec::new();
                for (c, b) in arms {
                    let hc = self.cond(c)?;
                    // "Se houver apelido:": dentro do bloco, "o apelido" é o valor presente.
                    let body = match (c, &hc) {
                        (ast::Cond::Atom { negated: false, kind: ast::CondKind::Exists { word, word_span }, .. }, hir::Cond::IsSome(src)) => {
                            self.scopes.push(Vec::new());
                            let Ty::Maybe(inner) = self.resolve_ty(&src.ty) else { unreachable!() };
                            let info = self.lookup(word).unwrap();
                            let name = ast::NameRef {
                                word: word.clone(),
                                span: *word_span,
                                word_span: *word_span,
                                article: Some(ast::ArticleUse { gender: info.gender, number: info.number, span: *word_span, contraction: None }),
                            };
                            let id = self.declare(&name, *inner.clone(), false)?;
                            self.locals[id].local.borrowed = true;
                            self.locals[id].local.display = name.display();
                            let unwrap = hir::Expr { kind: hir::ExprKind::Unwrap(Box::new(src.clone())), ty: *inner, span: *word_span };
                            let mut inner_block = self.block(b)?;
                            inner_block.0.insert(0, hir::Stmt::Let { var: Var::Local(id), value: unwrap });
                            self.scopes.pop();
                            inner_block
                        }
                        _ => self.block(b)?,
                    };
                    harms.push((hc, body));
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
            ast::Stmt::ForEach { item, source, body, .. } => {
                let src = match self.expr(source, None) {
                    Ok(e) => e,
                    Err(d) if d.category == Category::Scope && matches!(&source.kind, ExprKind::Place(p) if p.span == item.span) => {
                        let ExprKind::Place(p) = &source.kind else { unreachable!() };
                        return err(Diagnostic::new(Category::Scope, format!("\"Para cada {}\" procura \"{}\", que não foi declarado.", item.word, p.root.display()), item.span)
                            .hint(format!("Diga de onde vêm os itens: \"Para cada {} de ...:\".", item.word)));
                    }
                    Err(d) => return Err(d),
                };
                let elem = self.element_of(&src.ty, "a coleção", source.span)?;
                // O item herda o gênero da coleção quando ela é um nome no plural.
                let mut item = item.clone();
                if let ExprKind::Place(p) = &source.kind
                    && p.proj.is_empty()
                    && p.root.number() == Number::Plural
                    && let Some(a) = &mut item.article
                {
                    a.gender = p.root.gender();
                }
                self.scopes.push(Vec::new());
                let id = self.declare(&item, elem, false)?;
                self.locals[id].local.borrowed = true;
                let body = self.loop_body(body)?;
                self.scopes.pop();
                out.push(hir::Stmt::ForEach { var: id, source: src, body });
            }
            _ => return err(Diagnostic::new(Category::Syntax, "Construção ainda não suportada.", stmt_span(s))),
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
        self.check_name(name)?;
        let id = self.locals.len();
        self.locals.push(LocalInfo {
            local: hir::Local { name: name.word.clone(), display: name.display(), ty, mutable, borrowed: false, span: name.span },
            gender: name.gender(),
            number: name.number(),
            mutated: false,
            decl_span: name.span,
            hidden: false,
        });
        self.scopes.last_mut().unwrap().push((name.word.clone(), id));
        Ok(id)
    }

    fn check_name(&self, name: &ast::NameRef) -> Result<()> {
        if let Some(g) = self.env.globals.iter().find(|g| g.name == name.word) {
            return err(Diagnostic::new(Category::Scope, format!("\"{}\" já é uma constante do programa.", name.display()), name.span).label(g.span, "definida aqui"));
        }
        if !name.is_symbol() && name.word.chars().count() < 2 {
            return err(Diagnostic::new(Category::Morphology, "Um nome tem pelo menos duas letras.", name.span));
        }
        Ok(())
    }

    /// Guarda o valor num local escondido que "o resultado" retoma.
    fn set_result(&mut self, value: hir::Expr, out: &mut Vec<hir::Stmt>) {
        let id = self.locals.len();
        self.locals.push(LocalInfo {
            local: hir::Local { name: "resultado".into(), display: "o resultado".into(), ty: value.ty.clone(), mutable: false, borrowed: false, span: value.span },
            gender: Gender::Masculine,
            number: Number::Singular,
            mutated: true,
            decl_span: value.span,
            hidden: true,
        });
        out.push(hir::Stmt::Let { var: Var::Local(id), value });
        self.result = Some(id);
    }

    fn clause(&mut self, c: &ast::Clause, out: &mut Vec<hir::Stmt>) -> Result<()> {
        match &c.kind {
            ClauseKind::Decl { name, ann, mutable, init } => {
                let annotated = ann.as_ref().map(|a| self.env.type_of(a)).transpose()?;
                let plural = name.number() == Number::Plural;
                let value = match init {
                    Init::Expr(e) => {
                        let v = self.expr(e, annotated.as_ref())?;
                        let vt = self.resolve_ty(&v.ty);
                        if plural && !matches!(vt, Ty::List(_)) && annotated.is_none() {
                            // "As notas são 7.": coleção de um item só.
                            let span = v.span;
                            hir::Expr { ty: Ty::List(Box::new(vt)), kind: hir::ExprKind::List(vec![v]), span }
                        } else {
                            v
                        }
                    }
                    Init::List(items, span) => {
                        let mut elem = match annotated.as_ref().map(|t| self.resolve_ty(t)) {
                            Some(Ty::List(e)) => Some(*e),
                            Some(other) => {
                                return err(Diagnostic::new(Category::Type, format!("Vários valores formam uma lista, mas o aposto diz {}.", other.describe()), *span));
                            }
                            // "7, 8,5 e 9": havendo real escrito, os inteiros escritos viram reais.
                            None if items.iter().any(|i| matches!(i.kind, ExprKind::Real(_))) => Some(Ty::Real(64)),
                            None => None,
                        };
                        let mut out = Vec::new();
                        for it in items {
                            let v = self.expr(it, elem.as_ref())?;
                            if elem.is_none() {
                                elem = Some(v.ty.clone());
                            }
                            out.push(v);
                        }
                        hir::Expr { ty: Ty::List(Box::new(elem.unwrap())), kind: hir::ExprKind::List(out), span: *span }
                    }
                    Init::Empty(span) => {
                        let ty = match annotated.as_ref().map(|t| self.resolve_ty(t)) {
                            Some(t @ Ty::List(_)) => t,
                            Some(Ty::Text) => Ty::Text,
                            Some(other) => {
                                return err(Diagnostic::new(Category::Type, format!("Só lista e texto começam vazios, e o aposto diz {}.", other.describe()), *span));
                            }
                            None => {
                                let item = self.fresh(*span);
                                Ty::List(Box::new(item))
                            }
                        };
                        let kind = if ty == Ty::Text { hir::ExprKind::Text(String::new()) } else { hir::ExprKind::List(Vec::new()) };
                        hir::Expr { ty, kind, span: *span }
                    }
                };
                let ty = annotated.unwrap_or_else(|| value.ty.clone());
                let is_list = matches!(self.resolve_ty(&ty), Ty::List(_));
                if plural && !is_list {
                    return err(Diagnostic::new(Category::Agreement, format!("\"{}\" está no plural, então é uma coleção, mas o valor é {}.", name.display(), self.resolve_ty(&ty).describe()), name.span)
                        .hint("Nome no plural guarda vários itens; para um valor só, use o singular."));
                }
                if !plural && is_list && ann.is_none() {
                    let pl = format!("{} {}", morphology::definite_article(name.gender(), Number::Plural), morphology::pluralize(&name.word));
                    return err(Diagnostic::new(Category::Agreement, format!("\"{}\" está no singular, mas o valor é uma coleção.", name.display()), name.span)
                        .hint(format!("Use o plural (\"{pl}\") ou diga no aposto que é uma lista: \"{}, uma lista de inteiros, começa com ...\".", capitalize(&name.display()))));
                }
                if self.top {
                    self.check_name(name)?;
                    let id = self.env.globals.len();
                    self.env.globals.push(GlobalInfo { name: name.word.clone(), ty, gender: name.gender(), number: name.number(), span: name.span });
                    out.push(hir::Stmt::Let { var: Var::Global(id), value });
                } else {
                    let id = self.declare(name, ty, *mutable)?;
                    out.push(hir::Stmt::Let { var: Var::Local(id), value });
                }
                self.result = None;
            }
            ClauseKind::Push { place, value } => {
                let p = self.place_mut(place, &morphology::third_person("ganhar", place.number()).unwrap_or_default())?;
                let Ty::List(elem) = self.resolve_ty(&p.ty) else {
                    return err(Diagnostic::new(Category::Type, format!("Só lista ganha itens, e {} é {}.", place_name(place), self.resolve_ty(&p.ty).describe()), place.span));
                };
                let v = self.expr(value, Some(&elem))?;
                out.push(hir::Stmt::Push { place: p, value: v });
                self.result = None;
            }
            ClauseKind::Remove { place, which } | ClauseKind::Take { place, which } => {
                let take = matches!(c.kind, ClauseKind::Take { .. });
                let verb = if take { "dá itens".to_string() } else { morphology::third_person("perder", place.number()).unwrap_or_default() };
                let p = self.place_mut(place, &verb)?;
                let Ty::List(elem) = self.resolve_ty(&p.ty) else {
                    return err(Diagnostic::new(Category::Type, format!("Só lista perde itens, e {} é {}.", place_name(place), self.resolve_ty(&p.ty).describe()), place.span));
                };
                let index = match which {
                    ast::Which::Ordinal { position: 0, .. } => None,
                    ast::Which::Ordinal { position, span } => Some(Box::new(hir::Expr { kind: hir::ExprKind::Int(*position as i128), ty: Ty::Int(IntTy::I64), span: *span })),
                    ast::Which::Item(e) => Some(Box::new(self.expr(e, Some(&Ty::Int(IntTy::I64)))?)),
                };
                let e = hir::Expr { kind: hir::ExprKind::Take { place: p, index }, ty: *elem, span: c.span };
                if take {
                    self.set_result(e, out);
                } else {
                    out.push(hir::Stmt::Expr(e));
                    self.result = None;
                }
            }
            ClauseKind::Assign { place, value } => {
                let p = self.place_mut(place, "vira")?;
                let Init::Expr(e) = value else { unreachable!() };
                let v = self.expr(e, Some(&p.ty))?;
                out.push(hir::Stmt::Assign { place: p, value: v });
                self.result = None;
            }
            ClauseKind::Grow { place, value } | ClauseKind::Shrink { place, value } => {
                let grow = matches!(c.kind, ClauseKind::Grow { .. });
                let p = self.place_mut(place, if grow { "cresce" } else { "diminui" })?;
                if !p.ty.is_numeric() {
                    return err(Diagnostic::new(Category::Type, format!("Só número cresce ou diminui; {} é {}.", place_name(place), p.ty.describe()), place.span));
                }
                let v = self.expr(value, Some(&p.ty))?;
                let op = if grow { BinOp::Add } else { BinOp::Sub };
                out.push(hir::Stmt::Compound { place: p.clone(), op, value: v, span: c.span });
                let ty = p.ty.clone();
                let span = p.span;
                self.set_result(hir::Expr { kind: hir::ExprKind::Read(p), ty, span }, out);
            }
            ClauseKind::Write { parts, newline } => {
                let value = self.expr(&parts[0], None)?;
                if value.ty == Ty::Unit {
                    return err(Diagnostic::new(Category::Type, "Esse valor não existe: a função não devolve nada.", parts[0].span));
                }
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
                let ty = self.env.type_of(ty)?;
                self.set_result(hir::Expr { kind: hir::ExprKind::Input, ty, span: c.span }, out);
            }
            ClauseKind::Desist(msg) => {
                self.require_fallible(c.span)?;
                let m = self.expr(msg, Some(&Ty::Text))?;
                out.push(hir::Stmt::Desist(m));
            }
            ClauseKind::Break | ClauseKind::Continue => {
                let brk = matches!(c.kind, ClauseKind::Break);
                if self.loops == 0 {
                    let w = if brk { "Paro" } else { "Pulo" };
                    return err(Diagnostic::new(Category::Syntax, format!("\"{w}\" só faz sentido dentro de um laço."), c.span));
                }
                out.push(if brk { hir::Stmt::Break } else { hir::Stmt::Continue });
            }
            ClauseKind::Return(value) => {
                let sig = &self.env.prog.sigs[self.sig.unwrap()];
                let ret = self.env.sig_types[self.sig.unwrap()].ret.clone();
                let v = match (value, &ret) {
                    (None, Ty::Unit) => None,
                    (Some(e), Ty::Unit) => {
                        let what = if sig.is_main { "\"Para começar\" não devolve valor".to_string() } else { format!("\"{}\" não devolve valor", sig.infinitive) };
                        return err(Diagnostic::new(Category::Type, format!("{what}; aqui basta \"Devolvo.\""), e.span)
                            .maybe_hint((!sig.is_main).then(|| "Para devolver, acrescente \", devolvendo um ...\" ao cabeçalho.".to_string())));
                    }
                    (None, t) => {
                        return err(Diagnostic::new(Category::Type, format!("\"{}\" devolve {}: diga qual.", sig.infinitive, t.describe()), c.span));
                    }
                    (Some(e), t) => Some(self.expr(e, Some(t))?),
                };
                out.push(hir::Stmt::Return(v));
            }
            ClauseKind::Call { call, handler } => {
                let e = match handler {
                    None => self.call(call)?,
                    Some(h) => {
                        if !self.env.prog.sigs[call.sig].fallible {
                            return err(Diagnostic::new(Category::Syntax, format!("\"{}\" não falha, então não precisa de \", ou ...\".", self.env.prog.sigs[call.sig].infinitive), c.span));
                        }
                        self.handled = true;
                        let inner = self.call(call);
                        self.handled = false;
                        let inner = inner?;
                        let ty = inner.ty.clone();
                        let fb = self.fallback(h, &ty, c.span)?;
                        hir::Expr { kind: hir::ExprKind::Try { call: Box::new(inner), fallback: fb }, ty, span: c.span }
                    }
                };
                if e.ty == Ty::Unit {
                    out.push(hir::Stmt::Expr(e));
                    self.result = None;
                } else {
                    self.set_result(e, out);
                }
            }
        }
        Ok(())
    }

    fn call(&mut self, call: &ast::Call) -> Result<hir::Expr> {
        let sig = &self.env.prog.sigs[call.sig];
        let handled = std::mem::take(&mut self.handled);
        if sig.fallible && !handled {
            let usage = capitalize(&sig.usage());
            return err(Diagnostic::new(Category::Type, format!("\"{}\" pode falhar, e a chamada não diz o que fazer se falhar.", sig.infinitive), call.span)
                .label(sig.name_span, "definido com \"salvo uma falha\"")
                .hint(format!("Passe a falha adiante (\"{usage}, ou desisto.\") ou dê um valor de reserva (\"..., ou 0\").")));
        }
        let types = self.env.sig_types[call.sig].params.clone();
        let ret = self.env.sig_types[call.sig].ret.clone();
        let mut args = Vec::new();
        for ((a, p), ty) in call.args.iter().zip(&sig.params).zip(&types) {
            let place = if p.mode == ParamMode::Lent {
                let ExprKind::Place(pl) = &a.kind else {
                    return err(Diagnostic::new(Category::Borrow, format!("\"{}\" recebe {} confiado: precisa de um lugar que possa mudar, não de um valor solto.", sig.infinitive, p.name.display()), a.span)
                        .label(p.name.span, "confiado aqui"));
                };
                let hp = self.place_mut_for(pl, || format!("não se confia a \"{}\"", sig.infinitive))?;
                Some(hp)
            } else {
                None
            };
            let value = self.expr(a, Some(ty))?;
            args.push(hir::Arg { value, param: p.name.display(), mode: p.mode, place });
        }
        Ok(hir::Expr { kind: hir::ExprKind::Call { func: self.env.sig_func[call.sig], args }, ty: ret, span: call.span })
    }

    fn cond(&mut self, c: &ast::Cond) -> Result<hir::Cond> {
        Ok(match c {
            ast::Cond::And(v) => hir::Cond::And(v.iter().map(|c| self.cond(c)).collect::<Result<_>>()?),
            ast::Cond::Or(v) => hir::Cond::Or(v.iter().map(|c| self.cond(c)).collect::<Result<_>>()?),
            ast::Cond::Atom { negated, kind, span } => {
                let inner = match kind {
                    ast::CondKind::Rel { subject, rel, rhs } => {
                        let lhs = self.expr(subject, None)?;
                        let lt = lhs.ty.clone();
                        let rhs = self.expr(rhs, Some(&lt))?;
                        let ordered = !matches!(rel, Rel::Equal | Rel::Different);
                        if ordered && !matches!(lhs.ty, Ty::Int(_) | Ty::Real(_) | Ty::Char | Ty::Text | Ty::Error) {
                            return err(Diagnostic::new(Category::Type, format!("\"{}\" compara números, caracteres ou textos, e este valor é {}.", rel.words(), lhs.ty.describe()), *span));
                        }
                        hir::Cond::Cmp { rel: *rel, lhs, rhs }
                    }
                    ast::CondKind::Pred { call } => hir::Cond::Test(self.call(call)?),
                    ast::CondKind::Exists { word, word_span } => {
                        let e = self.maybe_named(word, *word_span)?;
                        hir::Cond::IsSome(e)
                    }
                    _ => return err(Diagnostic::new(Category::Syntax, "Condição ainda não suportada.", *span)),
                };
                if *negated { hir::Cond::Not(Box::new(inner)) } else { inner }
            }
        })
    }

    /// O nome de um "Se houver apelido": tem que ser um "talvez".
    fn maybe_named(&mut self, word: &str, span: Span) -> Result<hir::Expr> {
        let Some(info) = self.lookup(word) else {
            return err(Diagnostic::new(Category::Scope, format!("\"{word}\" não foi declarado."), span));
        };
        let ty = self.resolve_ty(&info.ty);
        if !matches!(ty, Ty::Maybe(_)) {
            let name = format!("{} {word}", morphology::definite_article(info.gender, info.number));
            return err(Diagnostic::new(Category::Type, format!("\"{name}\" é {} e sempre existe; \"houver\" é para o que foi declarado com \"talvez\".", ty.describe()), span));
        }
        Ok(hir::Expr { kind: hir::ExprKind::Read(hir::Place::var(info.var, ty.clone(), span)), ty, span })
    }

    fn resolve(&self, name: &ast::NameRef) -> Result<NameInfo> {
        let Some(info) = self.lookup(&name.word) else {
            let mut d = Diagnostic::new(Category::Scope, format!("\"{}\" não foi declarado.", name.display()), name.span);
            let near = self
                .scopes
                .iter()
                .flatten()
                .map(|(w, _)| w.as_str())
                .chain(self.env.globals.iter().map(|g| g.name.as_str()))
                .filter(|w| morphology::edit_distance(w, &name.word) <= 2)
                .min_by_key(|w| morphology::edit_distance(w, &name.word));
            if let Some(w) = near {
                d = d.hint(format!("Quis dizer \"{w}\"?"));
            } else {
                d = d.hint(format!("Declare antes de usar: \"{} é ...\" ou \"{} começa em ...\".", capitalize(&name.display()), capitalize(&name.display())));
            }
            return err(d);
        };
        if let Some(a) = &name.article
            && (a.gender != info.gender || a.number != info.number)
        {
            let right = morphology::definite_article(info.gender, info.number);
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
                format!("\"{written}\" não concorda: \"{}\" é {}. Escreva \"{fixed}\".", name.word, morphology::describe_gn(info.gender, info.number)),
                name.span,
            )
            .label(info.decl_span, "declarado aqui"));
        }
        Ok(info)
    }

    fn place_read(&mut self, p: &ast::Place) -> Result<(hir::Place, NameInfo)> {
        let info = self.resolve(&p.root)?;
        let mut ty = self.resolve_ty(&info.ty);
        let mut proj = Vec::new();
        let mut owner = p.root.display();
        for pr in &p.proj {
            match pr {
                ast::Proj::Field(n) => {
                    let Ty::Record(r) = &ty else {
                        return err(Diagnostic::new(Category::Type, format!("\"{}\" é {} e não tem campos.", owner, ty.describe()), n.span));
                    };
                    let Some(i) = r.fields.iter().position(|(f, _)| *f == n.word) else {
                        let names: Vec<String> = r.fields.iter().map(|(f, _)| format!("\"{f}\"")).collect();
                        return err(Diagnostic::new(Category::Scope, format!("{} não tem o campo \"{}\".", capitalize(&Ty::Record(r.clone()).describe()), n.word), n.word_span)
                            .hint(format!("Os campos são: {}.", names.join(", "))));
                    };
                    let def = &self.env.prog.records[r.id].fields[i];
                    if let Some(a) = n.article
                        && (a.gender != def.name.gender() || a.number != def.name.number())
                    {
                        return err(Diagnostic::new(Category::Agreement, format!("\"{}\" não concorda com o campo, que é \"{}\".", n.display(), def.name.display()), n.span));
                    }
                    owner = format!("{} de {owner}", n.display());
                    let fty = r.fields[i].1.clone();
                    proj.push(hir::PlaceProj::Field(i));
                    ty = fty;
                }
                ast::Proj::Index(e, span) => {
                    let elem = self.element_of(&ty, &owner, *span)?;
                    let idx = self.expr(e, Some(&Ty::Int(IntTy::I64)))?;
                    proj.push(hir::PlaceProj::Index(Box::new(idx), *span));
                    ty = elem;
                }
                ast::Proj::Ordinal { position, span, .. } => {
                    let elem = self.element_of(&ty, &owner, *span)?;
                    if *position == 0 {
                        proj.push(hir::PlaceProj::Last(*span));
                    } else {
                        let idx = hir::Expr { kind: hir::ExprKind::Int(*position as i128), ty: Ty::Int(IntTy::I64), span: *span };
                        proj.push(hir::PlaceProj::Index(Box::new(idx), *span));
                    }
                    ty = elem;
                }
            }
        }
        Ok((hir::Place { root: info.var, proj, ty, span: p.span }, info))
    }

    /// Tipo do item de uma coleção (lista ou texto).
    fn element_of(&self, ty: &Ty, owner: &str, span: Span) -> Result<Ty> {
        match self.resolve_ty(ty) {
            Ty::List(e) => Ok(*e),
            Ty::Text => Ok(Ty::Char),
            Ty::Infer(_) => err(Diagnostic::new(Category::Type, format!("Ainda não sei de que é \"{owner}\" para tirar um item."), span)),
            other => err(Diagnostic::new(Category::Type, format!("\"{owner}\" é {} e não tem itens.", other.describe()), span)),
        }
    }

    fn place_mut(&mut self, p: &ast::Place, verb: &str) -> Result<hir::Place> {
        self.place_mut_for(p, || format!("não {verb}"))
    }

    fn place_mut_for(&mut self, p: &ast::Place, what: impl FnOnce() -> String) -> Result<hir::Place> {
        let (hp, info) = self.place_read(p)?;
        if !info.mutable {
            let name = place_name(p);
            let be = morphology::third_person("ser", info.number).unwrap_or_default();
            let fixed = morphology::inflect_adjective("fixo", Gender::Masculine, info.gender, info.number);
            let cap = capitalize(&name);
            let (message, label, hint) = match info.var {
                Var::Global(_) => {
                    let constant = if info.number == Number::Plural { "constantes do programa" } else { "uma constante do programa" };
                    (format!("{cap} {be} {constant} e {}.", what()), "definida no nível de fora".to_string(), None)
                }
                Var::Local(id) if self.is_param(id) => {
                    let sig = &self.env.prog.sigs[self.sig.unwrap()];
                    let param = &sig.params[id];
                    let ty_gender = Gender::Masculine;
                    let lent = morphology::inflect_adjective("confiado", Gender::Masculine, ty_gender, Number::Singular);
                    let message = match param.mode {
                        ParamMode::Borrowed => {
                            let borrowed = morphology::inflect_adjective("emprestado", Gender::Masculine, info.gender, info.number);
                            format!("{cap} chega {borrowed} só para leitura e {}.", what())
                        }
                        _ => format!("{cap} {be} parâmetro sem \"confiado\" e {}: a função recebeu o valor, não o lugar.", what()),
                    };
                    (
                        message,
                        "parâmetro sem \"confiado\"".to_string(),
                        Some(format!("Para a função poder mudar o valor de quem chama, declare o parâmetro como \"{lent}\" no cabeçalho.")),
                    )
                }
                Var::Local(_) => {
                    let begin = morphology::third_person("começar", info.number).unwrap_or_default();
                    (
                        format!("{cap} {be} {fixed} por definição e {}.", what()),
                        format!("definido com \"{be}\""),
                        Some(format!("Quem varia se declara com \"{begin} em\": \"{cap} {begin} em ...\".")),
                    )
                }
            };
            return err(Diagnostic::new(Category::Mutability, message, p.span).label(info.decl_span, label).maybe_hint(hint));
        }
        if let Var::Local(id) = info.var {
            self.locals[id].mutated = true;
        }
        if hp.proj.iter().any(|p| !matches!(p, hir::PlaceProj::Field(_))) && hp.ty == Ty::Char {
            return err(Diagnostic::new(Category::Type, "Texto não se muda letra por letra.", p.span)
                .hint("Monte o texto novo com \"seguido de\"."));
        }
        Ok(hp)
    }

    fn is_param(&self, id: hir::LocalId) -> bool {
        self.sig.is_some_and(|s| id < self.env.prog.sigs[s].params.len())
    }

    fn expr(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Result<hir::Expr> {
        let (kind, ty) = match &e.kind {
            ExprKind::Int(v) => {
                let ty = match expected {
                    Some(Ty::Int(i)) => Ty::Int(*i),
                    Some(Ty::Real(b)) => {
                        let r = if *b == 32 { *v as f32 as f64 } else { *v as f64 };
                        return Ok(hir::Expr { kind: hir::ExprKind::Real(r), ty: Ty::Real(*b), span: e.span });
                    }
                    _ => Ty::Int(IntTy::I64),
                };
                if let Ty::Int(i) = ty
                    && !i.contains(*v)
                {
                    return err(Diagnostic::new(Category::Type, format!("{v} não cabe num {}: vai de {} a {}.", i.noun(false), i.min(), i.max()), e.span));
                }
                (hir::ExprKind::Int(*v), ty)
            }
            ExprKind::Real(v) => {
                let bits = match expected {
                    Some(Ty::Real(b)) => *b,
                    _ => 64,
                };
                // Real de 32 bits guarda o valor arredondado a 32 bits.
                let v = if bits == 32 { *v as f32 as f64 } else { *v };
                (hir::ExprKind::Real(v), Ty::Real(bits))
            }
            ExprKind::Text(s) => (hir::ExprKind::Text(s.clone()), Ty::Text),
            ExprKind::Char(c) => (hir::ExprKind::Char(*c), Ty::Char),
            ExprKind::Bool(b) => (hir::ExprKind::Bool(*b), Ty::Bool),
            ExprKind::Place(p) => {
                let (hp, _) = self.place_read(p)?;
                let ty = hp.ty.clone();
                (hir::ExprKind::Read(hp), ty)
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let num_hint = expected.filter(|t| t.is_numeric());
                let l = self.expr(lhs, num_hint)?;
                let r_hint = if matches!(op, BinOp::Shl | BinOp::Shr) { None } else { Some(&l.ty).filter(|t| t.is_numeric()) };
                let r_hint = r_hint.cloned();
                let r = self.expr(rhs, r_hint.as_ref())?;
                let ty = self.binary_type(*op, &l, &r, e.span)?;
                (hir::ExprKind::Binary(*op, Box::new(l), Box::new(r)), ty)
            }
            ExprKind::Anaphora => {
                let Some(id) = self.result else {
                    return err(Diagnostic::new(Category::Scope, "\"O resultado\" retoma o valor da oração anterior do mesmo período, e aqui nenhuma oração antes deu valor.", e.span)
                        .hint("Junte as orações no mesmo período: \"Leio um inteiro e escrevo o resultado.\""));
                };
                let ty = self.locals[id].local.ty.clone();
                (hir::ExprKind::Read(hir::Place::var(Var::Local(id), ty.clone(), e.span)), ty)
            }
            ExprKind::Concat(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    let v = self.expr(p, None)?;
                    if v.ty == Ty::Unit {
                        return err(Diagnostic::new(Category::Type, "Esse valor não existe: a função não devolve nada.", p.span));
                    }
                    out.push(v);
                }
                (hir::ExprKind::Concat(out), Ty::Text)
            }
            ExprKind::Call(call) => {
                let c = self.call(call)?;
                if c.ty == Ty::Unit {
                    let sig = &self.env.prog.sigs[call.sig];
                    return err(Diagnostic::new(Category::Type, format!("\"{}\" não devolve valor, então não tem resultado.", sig.infinitive), call.verb_span)
                        .label(sig.name_span, "definido sem \"devolvendo\"")
                        .hint(format!("Use como oração: \"{}.\"", capitalize(&sig.usage()))));
                }
                (c.kind, c.ty)
            }
            ExprKind::Fact(c) => (hir::ExprKind::Fact(Box::new(self.cond(c)?)), Ty::Bool),
            ExprKind::Nothing => {
                let ty = match expected.map(|t| self.resolve_ty(t)) {
                    Some(t @ Ty::Maybe(_)) => t,
                    Some(Ty::Infer(_)) | None => {
                        let inner = self.fresh(e.span);
                        Ty::Maybe(Box::new(inner))
                    }
                    Some(other) => {
                        return err(Diagnostic::new(Category::Type, format!("\"nada\" só cabe onde se espera \"talvez\", e aqui se espera {}.", other.describe()), e.span)
                            .hint("Para um valor que pode faltar, declare com \"talvez\": \"O apelido, talvez um texto, começa em nada.\""));
                    }
                };
                (hir::ExprKind::Nothing, ty)
            }
            ExprKind::OrElse { value, fallback } => {
                self.handled = true;
                let call = self.expr(value, None);
                self.handled = false;
                let call = call?;
                let ExprKind::Call(c) = &value.kind else { unreachable!() };
                if !self.env.prog.sigs[c.sig].fallible {
                    return err(Diagnostic::new(Category::Syntax, format!("\"{}\" não falha, então não precisa de \", ou ...\".", self.env.prog.sigs[c.sig].infinitive), e.span));
                }
                let ty = call.ty.clone();
                let fb = self.fallback(fallback, &ty, e.span)?;
                (hir::ExprKind::Try { call: Box::new(call), fallback: fb }, ty)
            }
            ExprKind::Unary { op, operand } => {
                let hint = match op {
                    UnOp::Sqrt => Some(Ty::Real(64)),
                    UnOp::Neg | UnOp::Abs => expected.filter(|t| t.is_numeric()).cloned(),
                    _ => None,
                };
                let v = self.expr(operand, hint.as_ref())?;
                let vt = self.resolve_ty(&v.ty);
                let ty = match op {
                    UnOp::Len => match vt {
                        Ty::List(_) | Ty::Text => Ty::Int(IntTy::I64),
                        other => return err(Diagnostic::new(Category::Type, format!("Só lista e texto têm tamanho, e este valor é {}.", other.describe()), operand.span)),
                    },
                    UnOp::Neg | UnOp::Abs if vt.is_numeric() => vt,
                    UnOp::Sqrt if matches!(vt, Ty::Real(_)) => vt,
                    UnOp::BitNot if matches!(vt, Ty::Int(_)) => vt,
                    _ => {
                        let what = match op {
                            UnOp::Sqrt => "um real",
                            UnOp::BitNot => "um inteiro",
                            _ => "um número",
                        };
                        return err(Diagnostic::new(Category::Type, format!("A operação pede {what}, e este valor é {}.", vt.describe()), operand.span));
                    }
                };
                (hir::ExprKind::Unary(*op, Box::new(v)), ty)
            }
            ExprKind::Copy(inner) => {
                let v = self.expr(inner, expected)?;
                let ty = v.ty.clone();
                (hir::ExprKind::Copy(Box::new(v)), ty)
            }
            ExprKind::Construct { word, word_span, fields, .. } => {
                let r = self.env.records[word].clone();
                let mut values: Vec<Option<hir::Expr>> = vec![None; r.fields.len()];
                for (fname, fspan, value) in fields {
                    let Some(i) = r.fields.iter().position(|(f, _)| f == fname) else {
                        let names: Vec<String> = r.fields.iter().map(|(f, _)| format!("\"{f}\"")).collect();
                        return err(Diagnostic::new(Category::Scope, format!("{} não tem o campo \"{fname}\".", capitalize(&Ty::Record(r.clone()).describe())), *fspan)
                            .hint(format!("Os campos são: {}.", names.join(", "))));
                    };
                    if values[i].is_some() {
                        return err(Diagnostic::new(Category::Syntax, format!("O campo \"{fname}\" aparece duas vezes."), *fspan));
                    }
                    let fty = r.fields[i].1.clone();
                    let plural_field = self.env.prog.records[r.id].fields[i].name.number() == Number::Plural;
                    values[i] = Some(match (&fty, plural_field) {
                        // "as movimentações 100,0": campo no plural com um valor só.
                        (Ty::List(elem), true) if !matches!(value.kind, ExprKind::Place(_) | ExprKind::Call(_) | ExprKind::Copy(_)) => {
                            let v = self.expr(value, Some(elem))?;
                            let span = v.span;
                            hir::Expr { kind: hir::ExprKind::List(vec![v]), ty: fty.clone(), span }
                        }
                        _ => self.expr(value, Some(&fty))?,
                    });
                }
                let missing: Vec<String> = r.fields.iter().zip(&values).filter(|(_, v)| v.is_none()).map(|((f, _), _)| {
                    let d = &self.env.prog.records[r.id].fields.iter().find(|x| &x.name.word == f).unwrap().name;
                    format!("\"{}\"", d.display())
                }).collect();
                if !missing.is_empty() {
                    return err(Diagnostic::new(Category::Type, format!("{} precisa de todos os campos; faltou {}.", capitalize(&Ty::Record(r.clone()).describe()), join_e(&missing)), *word_span)
                        .hint("Nada começa sem valor: diga cada campo depois de \"com\"."));
                }
                (hir::ExprKind::Record(values.into_iter().map(Option::unwrap).collect()), Ty::Record(r))
            }
            _ => return err(Diagnostic::new(Category::Syntax, "Expressão ainda não suportada.", e.span)),
        };
        let mut out = hir::Expr { kind, ty, span: e.span };
        // Um valor onde se espera "talvez": é o valor presente.
        if let Some(Ty::Maybe(inner)) = expected.map(|t| self.resolve_ty(t))
            && !matches!(self.resolve_ty(&out.ty), Ty::Maybe(_))
            && self.unify(&inner, &out.ty)
        {
            let span = out.span;
            out = hir::Expr { kind: hir::ExprKind::Wrap(Box::new(out)), ty: Ty::Maybe(inner), span };
        }
        if let Some(exp) = expected
            && !self.unify(exp, &out.ty)
        {
            let (exp, got) = (self.resolve_ty(exp), self.resolve_ty(&out.ty));
            return err(Diagnostic::new(Category::Type, format!("Esperava {}, mas este valor é {}.", exp.describe(), got.describe()), e.span));
        }
        Ok(out)
    }

    /// O que fazer com a falha: valor de reserva, ou passar adiante (`None`).
    fn fallback(&mut self, f: &ast::Fallback, ty: &Ty, span: Span) -> Result<Option<Box<hir::Expr>>> {
        match f {
            ast::Fallback::Value(v) => {
                if *ty == Ty::Unit {
                    return err(Diagnostic::new(Category::Type, "A função não devolve valor, então não há valor de reserva: use \", ou desisto\".", v.span));
                }
                Ok(Some(Box::new(self.expr(v, Some(ty))?)))
            }
            ast::Fallback::Desist(msg) => {
                self.require_fallible(span)?;
                if msg.is_some() {
                    return err(Diagnostic::new(Category::Syntax, "\", ou desisto\" passa adiante a falha que veio; o motivo já é o dela.", span));
                }
                Ok(None)
            }
        }
    }

    fn require_fallible(&self, span: Span) -> Result<()> {
        let sig = &self.env.prog.sigs[self.sig.unwrap()];
        if !sig.fallible {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" não pode desistir: o cabeçalho não diz \"salvo uma falha\".", sig.infinitive), span)
                .label(sig.name_span, "definido sem \"salvo uma falha\"")
                .hint("Acrescente \", salvo uma falha\" ao cabeçalho, ou trate a falha aqui com \", ou VALOR\"."));
        }
        Ok(())
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
        let bits = matches!(op, BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr);
        if bits && !matches!(l.ty, Ty::Int(_)) {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" opera bits de inteiros, e este valor é {}.", op.words(), l.ty.describe()), l.span));
        }
        if matches!(op, BinOp::Shl | BinOp::Shr) {
            return Ok(l.ty.clone());
        }
        if l.ty != r.ty && !l.ty.is_error() && !r.ty.is_error() {
            return err(Diagnostic::new(Category::Type, format!("\"{}\" entre {} e {}: os dois lados precisam ter o mesmo tipo.", op.words(), l.ty.describe(), r.ty.describe()), span)
                .hint("Converta um dos lados com \"como\": \"o total como real\"."));
        }
        Ok(l.ty.clone())
    }
}

/// Todo caminho do bloco termina em "Devolvo"?
fn always_returns(b: &hir::Block) -> bool {
    b.0.iter().any(|s| match s {
        hir::Stmt::Return(_) | hir::Stmt::Desist(_) => true,
        hir::Stmt::If { arms, otherwise: Some(o) } => arms.iter().all(|(_, b)| always_returns(b)) && always_returns(o),
        hir::Stmt::Repeat { count: None, body } => !breaks(body),
        _ => false,
    })
}

fn breaks(b: &hir::Block) -> bool {
    b.0.iter().any(|s| match s {
        hir::Stmt::Break => true,
        hir::Stmt::If { arms, otherwise } => arms.iter().any(|(_, b)| breaks(b)) || otherwise.as_ref().is_some_and(breaks),
        _ => false,
    })
}

fn top_level_error(span: Span) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::new(Category::Syntax, "No nível de fora só se definem constantes (\"A taxa é 0,05.\"), funções e tipos.", span)
            .hint("O que o programa faz vai dentro de \"Para começar:\"."),
    )
}

fn stmt_span(s: &ast::Stmt) -> Span {
    match s {
        ast::Stmt::Period(p) => p.span,
        ast::Stmt::If { span, .. }
        | ast::Stmt::While { span, .. }
        | ast::Stmt::Repeat { span, .. }
        | ast::Stmt::ForRange { span, .. }
        | ast::Stmt::ForEach { span, .. }
        | ast::Stmt::Match { span, .. } => *span,
    }
}

fn readable(t: &Ty) -> bool {
    matches!(t, Ty::Int(_) | Ty::Real(_) | Ty::Bool | Ty::Char | Ty::Text | Ty::Error)
}

fn resolve(t: &Ty, subst: &[(Option<Ty>, Span)]) -> Ty {
    match t {
        Ty::Infer(i) => match &subst[*i as usize].0 {
            Some(t) => resolve(t, subst),
            None => t.clone(),
        },
        Ty::List(x) => Ty::List(Box::new(resolve(x, subst))),
        Ty::Maybe(x) => Ty::Maybe(Box::new(resolve(x, subst))),
        other => other.clone(),
    }
}

fn contains_infer(t: &Ty) -> bool {
    match t {
        Ty::Infer(_) => true,
        Ty::List(x) | Ty::Maybe(x) => contains_infer(x),
        _ => false,
    }
}

fn resolve_block(b: &mut hir::Block, s: &[(Option<Ty>, Span)]) {
    for st in &mut b.0 {
        resolve_stmt(st, s);
    }
}

fn resolve_stmt(st: &mut hir::Stmt, s: &[(Option<Ty>, Span)]) {
    use hir::Stmt::*;
    match st {
        Let { value, .. } | Write { value, .. } | Expr(value) => resolve_expr(value, s),
        Assign { place, value } | Compound { place, value, .. } | Push { place, value } => {
            resolve_place(place, s);
            resolve_expr(value, s);
        }
        If { arms, otherwise } => {
            for (c, b) in arms {
                resolve_cond(c, s);
                resolve_block(b, s);
            }
            if let Some(b) = otherwise {
                resolve_block(b, s);
            }
        }
        While { cond, body } => {
            resolve_cond(cond, s);
            resolve_block(body, s);
        }
        Repeat { count, body } => {
            if let Some(c) = count {
                resolve_expr(c, s);
            }
            resolve_block(body, s);
        }
        ForRange { from, to, body, .. } => {
            resolve_expr(from, s);
            resolve_expr(to, s);
            resolve_block(body, s);
        }
        ForEach { source, body, .. } => {
            resolve_expr(source, s);
            resolve_block(body, s);
        }
        Return(v) => {
            if let Some(v) = v {
                resolve_expr(v, s);
            }
        }
        Desist(v) => resolve_expr(v, s),
        Break | Continue => {}
    }
}

fn resolve_cond(c: &mut hir::Cond, s: &[(Option<Ty>, Span)]) {
    match c {
        hir::Cond::And(v) | hir::Cond::Or(v) => v.iter_mut().for_each(|c| resolve_cond(c, s)),
        hir::Cond::Not(c) => resolve_cond(c, s),
        hir::Cond::Cmp { lhs, rhs, .. } => {
            resolve_expr(lhs, s);
            resolve_expr(rhs, s);
        }
        hir::Cond::Test(e) | hir::Cond::IsSome(e) => resolve_expr(e, s),
    }
}

fn resolve_place(p: &mut hir::Place, s: &[(Option<Ty>, Span)]) {
    p.ty = resolve(&p.ty, s);
    for pr in &mut p.proj {
        if let hir::PlaceProj::Index(e, _) = pr {
            resolve_expr(e, s);
        }
    }
}

fn resolve_expr(e: &mut hir::Expr, s: &[(Option<Ty>, Span)]) {
    e.ty = resolve(&e.ty, s);
    use hir::ExprKind::*;
    match &mut e.kind {
        Read(p) => resolve_place(p, s),
        Binary(_, a, b) => {
            resolve_expr(a, s);
            resolve_expr(b, s);
        }
        Unary(_, a) | Copy(a) | Wrap(a) | Unwrap(a) => resolve_expr(a, s),
        Try { call, fallback } => {
            resolve_expr(call, s);
            if let Some(f) = fallback {
                resolve_expr(f, s);
            }
        }
        Concat(v) | List(v) | Record(v) => v.iter_mut().for_each(|e| resolve_expr(e, s)),
        Call { args, .. } => {
            for a in args {
                resolve_expr(&mut a.value, s);
                if let Some(p) = &mut a.place {
                    resolve_place(p, s);
                }
            }
        }
        Fact(c) => resolve_cond(c, s),
        Take { place, index } => {
            resolve_place(place, s);
            if let Some(i) = index {
                resolve_expr(i, s);
            }
        }
        Int(_) | Real(_) | Text(_) | Char(_) | Bool(_) | Input | Nothing => {}
    }
}

fn place_name(p: &ast::Place) -> String {
    p.root.display()
}

/// "a, b e c".
fn join_e(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} e {last}", init.join(", ")),
    }
}

pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
