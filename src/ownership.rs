//! Posse e empréstimo. Texto, lista e registro têm dono: entregar um deles (declarar
//! outro nome com ele, guardá-lo numa lista, passá-lo a um parâmetro sem particípio,
//! devolvê-lo) move o valor, e o nome antigo não se usa mais. Na mesma chamada, o que
//! vai confiado não vai de novo nem emprestado; durante um "Para cada", a coleção está
//! emprestada ao laço e não muda.
//!
//! A análise corre sobre a HIR de cada função, acompanhando o estado de cada local pelos
//! caminhos do programa (Se, laços, Paro).

use crate::ast::ParamMode;
use crate::diagnostics::{Category, Diagnostic, Result, err};
use crate::hir::{self, Cond, ExprKind, Place, PlaceProj, Stmt, Var};
use crate::morphology::{self, Gender, Number};
use crate::sema::capitalize;
use crate::span::Span;

pub fn check(prog: &hir::Program, src: &str) -> Result<()> {
    let lines = LineIndex::new(src);
    Checker::new(prog, &prog.init, &lines).run()?;
    for f in &prog.funcs {
        Checker::new(prog, f, &lines).run()?;
    }
    Ok(())
}

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(src: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    fn line(&self, span: Span) -> usize {
        match self.starts.binary_search(&(span.start as usize)) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum How {
    /// Entregue a uma função.
    Given(String),
    /// Passou para outro nome.
    Passed(String),
    /// Guardado numa coleção.
    Stored(String),
    Returned,
    Other,
}

impl How {
    /// "foi entregue a \"arquivar\"", com o particípio concordando com o nome.
    fn finite(&self, name: &str) -> String {
        match self {
            How::Given(f) => format!("foi entregue a \"{f}\""),
            How::Passed(to) => format!("passou para \"{to}\""),
            How::Stored(to) => format!("foi {} em \"{to}\"", agree("guardado", name)),
            How::Returned => format!("foi {}", agree("devolvido", name)),
            How::Other => "foi entregue".to_string(),
        }
    }

    /// "pode já ter sido entregue a \"arquivar\"".
    fn maybe(&self, name: &str) -> String {
        match self {
            How::Passed(to) => format!("pode já ter passado para \"{to}\""),
            other => other.finite(name).replacen("foi ", "pode já ter sido ", 1),
        }
    }
}

/// Como o valor saiu das mãos do nome.
#[derive(Clone, Debug, PartialEq)]
struct Move {
    span: Span,
    /// Como foi, sem o verbo auxiliar: "entregue a \"arquivar\"", "passado para \"o texto\"".
    how: How,
    /// Chamada que recebeu a posse, para a dica: (verbo, parâmetro, tipo).
    callee: Option<(String, String, String)>,
    /// O movimento veio da volta anterior de um laço.
    from_loop: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum St {
    Live,
    Moved(Move),
    /// Movido em algum caminho, não em todos.
    Maybe(Move),
}

/// Estado de todos os locais num ponto do programa; `None` é código inalcançável
/// (depois de "Devolvo", "Paro" ou "Pulo").
type State = Option<Vec<St>>;

/// Um empréstimo ativo: a coleção de um "Para cada", o valor de um "Conforme".
struct Loan {
    var: Var,
    span: Span,
    what: LoanKind,
}

#[derive(Clone, Copy, PartialEq)]
enum LoanKind {
    Loop,
    Match,
}

impl LoanKind {
    fn to_whom(self) -> &'static str {
        match self {
            LoanKind::Loop => "ao laço \"Para cada\"",
            LoanKind::Match => "ao \"Conforme\"",
        }
    }

    fn label(self) -> &'static str {
        match self {
            LoanKind::Loop => "emprestado ao laço aqui",
            LoanKind::Match => "emprestado ao \"Conforme\" aqui",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            LoanKind::Loop => "Junte as mudanças numa lista nova e aplique depois do laço.",
            LoanKind::Match => "Mude o valor depois do \"Conforme\", fora dos casos.",
        }
    }
}

struct Checker<'a> {
    prog: &'a hir::Program,
    func: &'a hir::Func,
    lines: &'a LineIndex,
    state: State,
    loans: Vec<Loan>,
    /// Estados que saem dos laços abertos por "Paro" e voltam ao topo por "Pulo".
    breaks: Vec<Vec<Vec<St>>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Use {
    /// O valor é entregue (move, se não for de cópia).
    Move,
    /// Só se lê.
    Read,
}

fn join_st(a: &St, b: &St) -> St {
    match (a, b) {
        (St::Live, St::Live) => St::Live,
        (St::Moved(m), St::Moved(_)) => St::Moved(m.clone()),
        (St::Maybe(m), _) | (_, St::Maybe(m)) | (St::Moved(m), St::Live) | (St::Live, St::Moved(m)) => St::Maybe(m.clone()),
    }
}

fn join(a: &State, b: &State) -> State {
    match (a, b) {
        (None, s) | (s, None) => s.clone(),
        (Some(x), Some(y)) => Some(x.iter().zip(y).map(|(p, q)| join_st(p, q)).collect()),
    }
}

impl<'a> Checker<'a> {
    fn new(prog: &'a hir::Program, func: &'a hir::Func, lines: &'a LineIndex) -> Self {
        Checker { prog, func, lines, state: Some(vec![St::Live; func.locals.len()]), loans: Vec::new(), breaks: Vec::new() }
    }

    fn run(mut self) -> Result<()> {
        self.block(&self.func.body)
    }

    fn display(&self, v: Var) -> String {
        match v {
            Var::Local(id) => self.func.locals[id].display.clone(),
            Var::Global(id) => self.prog.globals[id].display.clone(),
        }
    }

    fn block(&mut self, b: &hir::Block) -> Result<()> {
        for s in &b.0 {
            if self.state.is_none() {
                break;
            }
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let { var, value } => {
                let how = How::Passed(self.display(*var));
                self.expr(value, Use::Move, &how)?;
                if let (Var::Local(id), Some(st)) = (var, &mut self.state) {
                    st[*id] = St::Live;
                }
            }
            Stmt::Assign { place, value } => {
                let how = How::Passed(place_display(self, place));
                self.expr(value, Use::Move, &how)?;
                self.mutate(place, "mudar")?;
                if place.proj.is_empty()
                    && let (Var::Local(id), Some(st)) = (place.root, &mut self.state)
                {
                    st[id] = St::Live;
                }
            }
            Stmt::Compound { place, value, .. } => {
                self.expr(value, Use::Read, &How::Other)?;
                self.mutate(place, "mudar")?;
            }
            Stmt::Push { place, value } => {
                let how = How::Stored(place_display(self, place));
                self.expr(value, Use::Move, &how)?;
                self.mutate(place, "ganhar itens")?;
            }
            Stmt::Write { value, .. } => self.expr(value, Use::Read, &How::Other)?,
            Stmt::Match { scrutinee, arms, otherwise } => {
                self.expr(scrutinee, Use::Read, &How::Other)?;
                // Os campos vistos dentro dos braços são empréstimos do valor: ele não muda.
                let loan = match &scrutinee.kind {
                    ExprKind::Read(p) => Some(Loan { var: p.root, span: scrutinee.span, what: LoanKind::Match }),
                    _ => None,
                };
                let has_loan = loan.is_some();
                if let Some(l) = loan {
                    self.loans.push(l);
                }
                let start = self.state.clone();
                let mut out: State = None;
                for a in arms {
                    self.state = start.clone();
                    self.block(&a.body)?;
                    out = join(&out, &self.state);
                }
                self.state = start.clone();
                if let Some(b) = otherwise {
                    self.block(b)?;
                    out = join(&out, &self.state);
                } else if arms.is_empty() {
                    out = start;
                }
                self.state = out;
                if has_loan {
                    self.loans.pop();
                }
            }
            Stmt::Desist(v) => {
                self.expr(v, Use::Move, &How::Returned)?;
                self.state = None;
            }
            Stmt::Expr(e) => self.expr(e, Use::Move, &How::Other)?,
            Stmt::Return(v) => {
                if let Some(v) = v {
                    self.expr(v, Use::Move, &How::Returned)?;
                }
                self.state = None;
            }
            Stmt::Break => {
                if let (Some(st), Some(acc)) = (&self.state, self.breaks.last_mut()) {
                    acc.push(st.clone());
                }
                self.state = None;
            }
            Stmt::Continue => {
                if let (Some(st), Some(acc)) = (&self.state, self.breaks.last_mut()) {
                    acc.push(st.clone());
                }
                self.state = None;
            }
            Stmt::If { arms, otherwise } => {
                let start = self.state.clone();
                let mut out: State = None;
                for (c, b) in arms {
                    self.cond(c)?;
                    let after_cond = self.state.clone();
                    self.block(b)?;
                    out = join(&out, &self.state);
                    self.state = after_cond;
                }
                match otherwise {
                    Some(b) => {
                        self.block(b)?;
                        out = join(&out, &self.state);
                    }
                    None => out = join(&out, &self.state),
                }
                let _ = start;
                self.state = out;
            }
            Stmt::While { cond, body } => {
                self.cond(cond)?;
                self.looping(body, None, |me| me.cond(cond))?;
            }
            Stmt::Repeat { count, body } => {
                if let Some(c) = count {
                    self.expr(c, Use::Read, &How::Other)?;
                }
                self.looping(body, None, |_| Ok(()))?;
            }
            Stmt::ForRange { from, to, body, .. } => {
                self.expr(from, Use::Read, &How::Other)?;
                self.expr(to, Use::Read, &How::Other)?;
                self.looping(body, None, |_| Ok(()))?;
            }
            Stmt::ForEach { source, body, .. } => {
                self.expr(source, Use::Read, &How::Other)?;
                let loan = match &source.kind {
                    ExprKind::Read(p) => Some(Loan { var: p.root, span: source.span, what: LoanKind::Loop }),
                    _ => None,
                };
                self.looping(body, loan, |_| Ok(()))?;
            }
        }
        Ok(())
    }

    /// Analisa o corpo do laço duas vezes: a segunda já parte do estado que a primeira
    /// volta deixou, e acusa o que foi entregue numa volta e usado na seguinte.
    fn looping(&mut self, body: &hir::Block, loan: Option<Loan>, mut head: impl FnMut(&mut Self) -> Result<()>) -> Result<()> {
        let has_loan = loan.is_some();
        if let Some(l) = loan {
            self.loans.push(l);
        }
        let entry = self.state.clone();
        self.breaks.push(Vec::new());
        self.block(body)?;
        let mut back = self.state.clone();
        for s in self.breaks.pop().unwrap() {
            back = join(&back, &Some(s));
        }
        // O que foi entregue numa volta chega à próxima como "talvez entregue".
        let mut second = join(&entry, &mark_loop(&back));
        if let (Some(e), Some(s)) = (&entry, &mut second) {
            for (a, b) in e.iter().zip(s.iter_mut()) {
                if *a == St::Live
                    && let St::Maybe(m) = b
                {
                    m.from_loop = true;
                }
            }
        }
        self.state = second.clone();
        head(self)?;
        self.breaks.push(Vec::new());
        self.block(body)?;
        let mut out = join(&second, &self.state);
        for s in self.breaks.pop().unwrap() {
            out = join(&out, &Some(s));
        }
        self.state = out;
        if has_loan {
            self.loans.pop();
        }
        Ok(())
    }

    fn cond(&mut self, c: &Cond) -> Result<()> {
        match c {
            Cond::And(v) | Cond::Or(v) => {
                for c in v {
                    self.cond(c)?;
                }
            }
            Cond::Not(c) => self.cond(c)?,
            Cond::Cmp { lhs, rhs, .. } => {
                self.expr(lhs, Use::Read, &How::Other)?;
                self.expr(rhs, Use::Read, &How::Other)?;
            }
            Cond::Test(e) => self.expr(e, Use::Move, &How::Other)?,
            Cond::IsSome(e) | Cond::IsVariant(e, _) => self.expr(e, Use::Read, &How::Other)?,
        }
        Ok(())
    }

    /// Confere que o lugar ainda tem dono, e acusa o uso depois de entregue.
    fn usable(&self, place: &Place, span: Span) -> Result<()> {
        let Var::Local(id) = place.root else { return Ok(()) };
        let Some(st) = &self.state else { return Ok(()) };
        let (m, maybe) = match &st[id] {
            St::Live => return Ok(()),
            St::Moved(m) => (m, false),
            St::Maybe(m) => (m, true),
        };
        let name = self.display(place.root);
        let line = self.lines.line(m.span);
        let message = if m.from_loop {
            format!("{} {} numa volta anterior do laço e não volta.", capitalize(&name), m.how.finite(&name))
        } else if maybe {
            format!("{} {} na linha {line}, num dos caminhos até aqui.", capitalize(&name), m.how.maybe(&name))
        } else {
            format!("{} {} na linha {line} e não volta.", capitalize(&name), m.how.finite(&name))
        };
        let given = agree("entregue", &name);
        let mut d =
            Diagnostic::new(Category::Ownership, message, span).primary(format!("{} aqui depois de {given}", agree("usado", &name)));
        if m.span != span {
            d = d.label(m.span, format!("{given} aqui"));
        }
        d = d.note(format!("Depois de {given}, \"{name}\" não se usa mais."));
        let copy = format!("uma cópia {}", of_name(&name));
        d = match &m.callee {
            Some((verb, param, ty)) => d.hint(format!(
                "Se \"{verb}\" só precisa ler, peça emprestado no cabeçalho (\"sendo {param} {ty} {}\"), ou entregue {copy}: \"{} {copy}.\"",
                borrowed_word(ty),
                capitalize(&self.first_person_of(verb))
            )),
            None => d.hint(format!("Se precisa dos dois, entregue {copy}.")),
        };
        if m.from_loop {
            d = d.note("Cada volta do laço entrega de novo o que a anterior já entregou.");
        }
        err(d)
    }

    /// Mudança no lugar: precisa ter dono e não estar emprestado a um laço. `what` é o
    /// verbo no infinitivo com o complemento ("ganhar itens").
    fn first_person_of(&self, infinitive: &str) -> String {
        morphology::first_person(infinitive).unwrap_or_else(|| infinitive.to_string())
    }

    fn mutate(&self, place: &Place, what: &str) -> Result<()> {
        if !place.proj.is_empty() {
            self.usable(place, place.span)?;
        }
        if let Some(l) = self.loans.iter().find(|l| l.var == place.root) {
            let name = self.display(place.root);
            let are = morphology::third_person("estar", gn(&name).1).unwrap_or_default();
            let (inf, rest) = what.split_once(' ').unwrap_or((what, ""));
            let verb = format!("{} {rest}", verb3(inf, &name)).trim_end().to_string();
            return err(Diagnostic::new(
                Category::Borrow,
                format!(
                    "{} {are} {} {} da linha {} e não {verb} enquanto ele durar.",
                    capitalize(&name),
                    agree("emprestado", &name),
                    l.what.to_whom(),
                    self.lines.line(l.span)
                ),
                place.span,
            )
            .label(l.span, l.what.label())
            .hint(l.what.hint()));
        }
        Ok(())
    }

    fn expr(&mut self, e: &hir::Expr, use_: Use, how: &How) -> Result<()> {
        match &e.kind {
            ExprKind::Read(p) => {
                self.place_exprs(p)?;
                self.usable(p, e.span)?;
                if use_ == Use::Move && !e.ty.is_copy() {
                    self.move_out(p, e.span, how, None)?;
                }
            }
            ExprKind::Binary(_, a, b) => {
                self.expr(a, Use::Read, &How::Other)?;
                self.expr(b, Use::Read, &How::Other)?;
            }
            ExprKind::Unary(_, a) | ExprKind::Copy(a) | ExprKind::Unwrap(a) | ExprKind::Convert(a) => {
                self.expr(a, Use::Read, &How::Other)?
            }
            ExprKind::Wrap(a) => self.expr(a, use_, how)?,
            ExprKind::Try { call, fallback } => {
                self.expr(call, Use::Move, how)?;
                if let Some(f) = fallback {
                    self.expr(f, use_, how)?;
                }
            }
            ExprKind::Concat(v) => {
                for x in v {
                    self.expr(x, Use::Read, &How::Other)?;
                }
            }
            ExprKind::VariantField { value, .. } => self.expr(value, Use::Read, &How::Other)?,
            ExprKind::List(v) | ExprKind::Record(v) | ExprKind::Variant { fields: v, .. } => {
                for x in v {
                    self.expr(x, Use::Move, how)?;
                }
            }
            ExprKind::Call { func, args } => self.call(*func, args, e.span)?,
            ExprKind::Fact(c) => self.cond(c)?,
            ExprKind::Take { place, index } => {
                if let Some(i) = index {
                    self.expr(i, Use::Read, &How::Other)?;
                }
                self.place_exprs(place)?;
                self.usable(place, place.span)?;
                self.mutate(place, "perder itens")?;
            }
            ExprKind::Int(_)
            | ExprKind::Real(_)
            | ExprKind::Text(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_)
            | ExprKind::Input
            | ExprKind::Nothing => {}
        }
        Ok(())
    }

    fn place_exprs(&mut self, p: &Place) -> Result<()> {
        for pr in &p.proj {
            if let PlaceProj::Index(e, _) = pr {
                self.expr(e, Use::Read, &How::Other)?;
            }
        }
        Ok(())
    }

    fn move_out(&mut self, p: &Place, span: Span, how: &How, callee: Option<(String, String, String)>) -> Result<()> {
        let name = self.display(p.root);
        if let Var::Global(_) = p.root {
            return err(Diagnostic::new(
                Category::Ownership,
                format!("{} é constante do programa e não se entrega.", capitalize(&name)),
                span,
            )
            .hint(format!("Entregue uma cópia: \"uma cópia {}\".", of_name(&name))));
        }
        if !p.proj.is_empty() {
            let whole = place_display(self, p);
            let from_list = p.proj.iter().any(|x| !matches!(x, PlaceProj::Field(_)));
            let hint = if from_list {
                format!("Entregue uma cópia (\"uma cópia {}\"), ou tire o item da lista com \"Tomo\".", of_name(&whole))
            } else {
                format!("Entregue uma cópia: \"uma cópia {}\".", of_name(&whole))
            };
            return err(Diagnostic::new(
                Category::Ownership,
                format!("Não se tira \"{whole}\" de dentro de \"{name}\" entregando: o valor tem dono."),
                span,
            )
            .hint(hint));
        }
        if let Var::Local(id) = p.root
            && self.func.locals[id].borrowed
        {
            return err(Diagnostic::new(
                Category::Ownership,
                format!("{} aqui é só uma vista emprestada do valor de fora e não se entrega.", capitalize(&name)),
                span,
            )
            .hint(format!("Entregue uma cópia: \"uma cópia {}\".", of_name(&name))));
        }
        if let Some(l) = self.loans.iter().find(|l| l.var == p.root) {
            let are = morphology::third_person("estar", gn(&name).1).unwrap_or_default();
            return err(Diagnostic::new(
                Category::Borrow,
                format!(
                    "{} {are} {} ao laço \"Para cada\" e não se {} enquanto ele durar.",
                    capitalize(&name),
                    agree("emprestado", &name),
                    verb3("entregar", &name)
                ),
                span,
            )
            .label(l.span, "emprestado ao laço aqui"));
        }
        if let (Var::Local(id), Some(st)) = (p.root, &mut self.state) {
            st[id] = St::Moved(Move { span, how: how.clone(), callee, from_loop: false });
        }
        Ok(())
    }

    fn call(&mut self, func: hir::FuncId, args: &[hir::Arg], span: Span) -> Result<()> {
        let callee = &self.prog.funcs[func];
        // Empréstimos desta chamada: quem foi confiado ou emprestado, e como.
        let mut lent: Vec<(Var, ParamMode, &str, Span)> = Vec::new();
        for a in args {
            match a.mode {
                ParamMode::Owned => {
                    if let ExprKind::Read(p) = &a.value.kind {
                        self.place_exprs(p)?;
                        self.usable(p, a.value.span)?;
                        if !a.value.ty.is_copy() {
                            let ty = a.value.ty.describe();
                            let how = How::Given(callee.name.clone());
                            self.move_out(p, a.value.span, &how, Some((callee.name.clone(), a.param.clone(), ty)))?;
                        }
                    } else {
                        let how = How::Given(callee.name.clone());
                        self.expr(&a.value, Use::Move, &how)?;
                    }
                }
                ParamMode::Borrowed | ParamMode::Lent => {
                    self.expr(&a.value, Use::Read, &How::Other)?;
                    let root = match (&a.place, &a.value.kind) {
                        (Some(p), _) => Some(p.root),
                        (None, ExprKind::Read(p)) => Some(p.root),
                        _ => None,
                    };
                    if let Some(p) = &a.place {
                        self.mutate(p, "mudar")?;
                    }
                    if let Some(root) = root {
                        if let Some((_, mode0, param0, span0)) =
                            lent.iter().find(|(v, m, _, _)| *v == root && (*m == ParamMode::Lent || a.mode == ParamMode::Lent))
                        {
                            let name = self.display(root);
                            let first = agree(if *mode0 == ParamMode::Lent { "confiado" } else { "emprestado" }, &name);
                            let again = agree(if a.mode == ParamMode::Lent { "confiado" } else { "emprestado" }, &name);
                            let both_lent = *mode0 == ParamMode::Lent && a.mode == ParamMode::Lent;
                            let hint = if both_lent {
                                "Cada parâmetro confiado precisa de um lugar diferente."
                            } else {
                                "Passe uma cópia no lugar do emprestado: \"uma cópia ...\"."
                            };
                            return err(Diagnostic::new(
                                Category::Borrow,
                                format!(
                                    "{} já vai {first} a \"{}\" como \"{}\" e não pode ir {again} de novo, como \"{}\", na mesma chamada.",
                                    capitalize(&name),
                                    callee.name,
                                    param0,
                                    a.param
                                ),
                                a.value.span,
                            )
                            .label(*span0, format!("{first} aqui"))
                            .note("Quem é confiado a uma função fica só com ela até a chamada terminar.")
                            .hint(hint));
                        }
                        lent.push((root, a.mode, &a.param, a.value.span));
                    }
                }
            }
        }
        let _ = span;
        Ok(())
    }
}

fn mark_loop(s: &State) -> State {
    s.clone()
}

fn place_display(c: &Checker, p: &Place) -> String {
    let mut name = c.display(p.root);
    let mut ty_rec: Option<std::sync::Arc<crate::types::RecordTy>> = match p.root {
        Var::Local(id) => match &c.func.locals[id].ty {
            crate::types::Ty::Record(r) => Some(r.clone()),
            _ => None,
        },
        Var::Global(id) => match &c.prog.globals[id].ty {
            crate::types::Ty::Record(r) => Some(r.clone()),
            _ => None,
        },
    };
    for pr in &p.proj {
        match pr {
            PlaceProj::Field(i) => {
                let Some(r) = ty_rec.clone() else { break };
                let (f, t) = &r.fields[*i];
                let art = r.field_articles[*i].1;
                name = format!("{art} {f} {}", of_name(&name));
                ty_rec = match t {
                    crate::types::Ty::Record(r) => Some(r.clone()),
                    _ => None,
                };
            }
            PlaceProj::Index(..) => {
                name = format!("o item {}", of_name(&name));
                ty_rec = None;
            }
            PlaceProj::Last(_) => {
                name = format!("o último {}", of_name(&name));
                ty_rec = None;
            }
        }
    }
    name
}

/// "da carta", "do saldo da conta".
fn of_name(name: &str) -> String {
    match name.split_once(' ') {
        Some((art, rest)) if crate::morphology::article(art).is_some() => format!("{} {rest}", crate::morphology::join_prep("de", art)),
        _ => format!("de {name}"),
    }
}

/// Particípio concordando com o tipo ("um texto emprestado", "uma lista emprestada").
fn borrowed_word(ty_described: &str) -> &'static str {
    if ty_described.starts_with("uma ") { "emprestada" } else { "emprestado" }
}

/// Gênero e número de um nome com artigo ("as notas").
fn gn(name: &str) -> (Gender, Number) {
    let art = name.split(' ').next().unwrap_or("o");
    morphology::article(art).map(|a| (a.gender, a.number)).unwrap_or((Gender::Masculine, Number::Singular))
}

/// Particípio ou adjetivo (dado no masculino singular) concordando com o nome.
fn agree(masc: &str, name: &str) -> String {
    let (g, n) = gn(name);
    morphology::inflect_adjective(masc, Gender::Masculine, g, n)
}

/// Verbo na 3ª pessoa concordando com o nome ("ganha" ou "ganham").
fn verb3(inf: &str, name: &str) -> String {
    morphology::third_person(inf, gn(name).1).unwrap_or_default()
}
