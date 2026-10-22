//! Interpretador da HIR. É a referência de semântica: o nativo tem que produzir a mesma
//! saída, byte a byte.

use crate::ast::{BinOp, ParamMode, Rel, UnOp};
use crate::codes::*;
use crate::hir::{self, Cond, ExprKind, PlaceProj, Stmt, StmtKind, Var};
use crate::span::Span;
use crate::textfmt;
use crate::types::{IntTy, Ty};
use std::cmp::Ordering;
use std::io::{BufRead, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i128),
    Real(f64),
    Bool(bool),
    Char(char),
    Text(String),
    List(Vec<Value>),
    Record(Vec<Value>),
    /// O "nada" de um "talvez"; o valor presente é guardado como ele mesmo.
    Nothing,
    /// Caso de enumeração com os campos.
    Variant(usize, Vec<Value>),
    Unit,
}

/// Um passo do caminho até um lugar, já com as posições calculadas.
enum Step {
    Field(usize),
    /// Posição a partir de 0.
    Index(usize),
}

/// Falha de execução: o código, a mensagem e onde aconteceu.
#[derive(Debug)]
pub struct Fault {
    /// O código do erro de execução; a falha declarada pelo programa não tem.
    pub code: Option<Code>,
    pub message: String,
    pub span: Span,
    /// Falha declarada ("Desisto com ..."), que sobe até quem a trata.
    pub desist: bool,
}

impl Fault {
    /// O diagnóstico que mostra a falha, como o `lavrac run` mostra.
    pub fn diagnostic(&self) -> crate::diagnostics::Diagnostic {
        match self.code {
            Some(code) if !self.desist => crate::diagnostics::Diagnostic::new(code, self.message.clone(), self.span),
            _ => crate::diagnostics::Diagnostic::failure(self.message.clone(), self.span).primary("o programa desistiu aqui"),
        }
    }

    /// O código de saída do processo: 101 no erro de execução, 1 na falha declarada.
    pub fn exit_code(&self) -> u8 {
        if self.code.is_some() && !self.desist { 101 } else { 1 }
    }
}

type Exec<T> = std::result::Result<T, Fault>;

/// Como um bloco terminou.
enum Flow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

const MAX_DEPTH: usize = textfmt::MAX_DEPTH as usize;

/// Um quadro da pilha: a função (o nível de fora não tem), onde ela foi chamada e os
/// valores dos locais.
#[derive(Debug)]
pub struct Frame {
    pub func: Option<hir::FuncId>,
    pub call_span: Span,
    pub values: Vec<Value>,
}

/// O que se vê da máquina num ponto da execução: o programa, a pilha (o quadro de cima é
/// o último) e as constantes.
pub struct View<'a> {
    pub prog: &'a hir::Program,
    pub frames: &'a [Frame],
    pub globals: &'a [Value],
}

/// Quem acompanha a execução, como o depurador: a máquina avisa antes de cada oração e
/// na falha, com a pilha à vista. Parar num ponto é não voltar de `before` ainda.
pub trait Tracer {
    /// Antes da oração `stmt`. Devolve `false` para encerrar a execução ali.
    fn before(&mut self, stmt: &Stmt, view: &View) -> bool;
    /// Uma falha de execução onde ela aconteceu, antes de a pilha se desfazer.
    fn fault(&mut self, fault: &Fault, view: &View);
}

pub struct Machine<'p, W: Write, R: BufRead> {
    prog: &'p hir::Program,
    out: W,
    input: R,
    globals: Vec<Value>,
    frames: Vec<Frame>,
    tracer: Option<Box<dyn Tracer + 'p>>,
    /// O acompanhante encerrou a execução.
    stopped: bool,
    /// A falha já foi mostrada ao acompanhante (ela ainda sobe pelos blocos de fora).
    fault_seen: bool,
    /// Ecoa a entrada lida na saída (útil quando a entrada vem de arquivo).
    pub echo: bool,
}

impl<'p, W: Write, R: BufRead> Machine<'p, W, R> {
    pub fn new(prog: &'p hir::Program, out: W, input: R) -> Self {
        Machine {
            prog,
            out,
            input,
            globals: vec![Value::Unit; prog.globals.len()],
            frames: Vec::new(),
            tracer: None,
            stopped: false,
            fault_seen: false,
            echo: false,
        }
    }

    /// Acompanha a execução com `tracer` (o depurador).
    pub fn trace(&mut self, tracer: Box<dyn Tracer + 'p>) {
        self.tracer = Some(tracer);
    }

    /// A execução terminou porque o acompanhante pediu.
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    pub fn run(&mut self) -> Exec<()> {
        self.frames.push(Frame { func: None, call_span: Span::default(), values: vec![Value::Unit; self.prog.init.locals.len()] });
        let r = self.block(&self.prog.init.body);
        self.frames.pop();
        let r = r.and_then(|_| {
            self.call(self.prog.main.expect("programa sem \"Para começar\" não chega ao interpretador"), Vec::new(), Span::default())
        });
        let _ = self.out.flush();
        match r {
            Ok((Err(f), _)) => Err(f),
            Ok(_) => Ok(()),
            Err(f) => Err(f),
        }
    }

    fn frame(&self) -> &[Value] {
        &self.frames.last().expect("a pilha tem quadro").values
    }

    fn frame_mut(&mut self) -> &mut [Value] {
        &mut self.frames.last_mut().expect("a pilha tem quadro").values
    }

    /// Chama a função. O resultado de dentro é o valor devolvido ou a falha declarada;
    /// o quadro volta junto para os parâmetros confiados.
    fn call(&mut self, func: hir::FuncId, args: Vec<Value>, span: Span) -> Exec<(Result<Value, Fault>, Vec<Value>)> {
        let f = &self.prog.funcs[func];
        if self.frames.len() >= MAX_DEPTH {
            return fault(L0914, textfmt::stack_overflow(), span);
        }
        let mut values = vec![Value::Unit; f.locals.len()];
        for ((id, _), v) in f.params.iter().zip(args) {
            values[*id] = v;
        }
        self.frames.push(Frame { func: Some(func), call_span: span, values });
        let r = self.block(&f.body);
        let frame = self.frames.pop().expect("o quadro da chamada").values;
        let ret = match r {
            Ok(Flow::Return(v)) => Ok(v),
            Ok(_) => Ok(Value::Unit),
            Err(f) if f.desist => Err(f),
            Err(f) => return Err(f),
        };
        Ok((ret, frame))
    }

    /// Parâmetro confiado: o valor final volta para o lugar do chamador. Como o
    /// empréstimo é exclusivo, copiar na ida e na volta equivale a passar a referência.
    fn copy_back(&mut self, func: hir::FuncId, args: &[hir::Arg], callee: &[Value]) -> Exec<()> {
        let f = &self.prog.funcs[func];
        for (a, (id, mode)) in args.iter().zip(&f.params) {
            if *mode == ParamMode::Lent
                && let Some(p) = &a.place
            {
                self.write_place(p, callee[*id].clone())?;
            }
        }
        Ok(())
    }

    fn block(&mut self, b: &hir::Block) -> Exec<Flow> {
        let mut prev = None;
        for s in &b.0 {
            // A língua às vezes faz de uma oração duas (o "o resultado" escondido, com o
            // mesmo trecho): o acompanhante vê uma só.
            let repeated = prev == Some(s.span.start);
            prev = Some(s.span.start);
            if !repeated
                && !s.span.is_empty()
                && let Some(t) = self.tracer.as_mut()
            {
                let view = View { prog: self.prog, frames: &self.frames, globals: &self.globals };
                if !t.before(s, &view) {
                    self.stopped = true;
                    return Err(Fault { code: None, message: String::new(), span: s.span, desist: false });
                }
            }
            match self.stmt(s) {
                Ok(Flow::Normal) => {}
                Ok(other) => return Ok(other),
                Err(f) => {
                    if !f.desist
                        && !self.fault_seen
                        && !self.stopped
                        && let Some(t) = self.tracer.as_mut()
                    {
                        self.fault_seen = true;
                        let view = View { prog: self.prog, frames: &self.frames, globals: &self.globals };
                        t.fault(&f, &view);
                    }
                    return Err(f);
                }
            }
        }
        Ok(Flow::Normal)
    }

    fn get(&self, v: Var) -> Value {
        match v {
            Var::Local(id) => self.frame()[id].clone(),
            Var::Global(id) => self.globals[id].clone(),
        }
    }

    fn set(&mut self, v: Var, value: Value) {
        match v {
            Var::Local(id) => self.frame_mut()[id] = value,
            Var::Global(id) => self.globals[id] = value,
        }
    }

    /// Calcula o caminho de um lugar, conferindo cada posição contra o tamanho da coleção.
    fn path(&mut self, p: &hir::Place) -> Exec<Vec<Step>> {
        let mut steps = Vec::new();
        for pr in &p.proj {
            match pr {
                PlaceProj::Field(i) => steps.push(Step::Field(*i)),
                PlaceProj::Index(e, span) => {
                    let Value::Int(n) = self.expr(e)? else { unreachable!() };
                    let len = self.len_at(p.root, &steps);
                    if n < 1 || n as usize > len {
                        return fault(L0909, position_error(n, len), *span);
                    }
                    steps.push(Step::Index(n as usize - 1));
                }
                PlaceProj::Last(span) => {
                    let len = self.len_at(p.root, &steps);
                    if len == 0 {
                        return fault(L0910, textfmt::EMPTY_LAST, *span);
                    }
                    steps.push(Step::Index(len - 1));
                }
            }
        }
        Ok(steps)
    }

    fn len_at(&self, root: Var, steps: &[Step]) -> usize {
        let base = match root {
            Var::Local(id) => &self.frame()[id],
            Var::Global(id) => &self.globals[id],
        };
        match walk(base, steps) {
            Value::List(v) => v.len(),
            Value::Text(t) => t.chars().count(),
            _ => 0,
        }
    }

    fn read_place(&mut self, p: &hir::Place) -> Exec<Value> {
        let steps = self.path(p)?;
        let base = match p.root {
            Var::Local(id) => &self.frame()[id],
            Var::Global(id) => &self.globals[id],
        };
        // A posição num texto é a de um caractere.
        if let (Some(Step::Index(i)), Value::Text(t)) = (steps.last(), walk(base, &steps[..steps.len().saturating_sub(1)])) {
            return Ok(Value::Char(t.chars().nth(*i).unwrap()));
        }
        Ok(walk(base, &steps).clone())
    }

    fn place_mut(&mut self, root: Var, steps: &[Step]) -> &mut Value {
        let base = match root {
            Var::Local(id) => &mut self.frames.last_mut().expect("a pilha tem quadro").values[id],
            Var::Global(id) => &mut self.globals[id],
        };
        walk_mut(base, steps)
    }

    fn write_place(&mut self, p: &hir::Place, v: Value) -> Exec<()> {
        let steps = self.path(p)?;
        *self.place_mut(p.root, &steps) = v;
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Exec<Flow> {
        match &s.kind {
            StmtKind::Let { var, value } => {
                let v = self.expr(value)?;
                self.set(*var, v);
            }
            StmtKind::Assign { place, value } => {
                let v = self.expr(value)?;
                self.write_place(place, v)?;
            }
            StmtKind::Compound { place, op, value, span } => {
                let cur = self.read_place(place)?;
                let v = self.expr(value)?;
                let r = arith(*op, cur, v, &place.ty, *span)?;
                self.write_place(place, r)?;
            }
            StmtKind::Push { place, value } => {
                let v = self.expr(value)?;
                let steps = self.path(place)?;
                let Value::List(list) = self.place_mut(place.root, &steps) else { unreachable!() };
                list.push(v);
            }
            StmtKind::ForEach { var, source, body } => {
                let items: Vec<Value> = match self.expr(source)? {
                    Value::List(v) => v,
                    Value::Text(t) => t.chars().map(Value::Char).collect(),
                    _ => unreachable!(),
                };
                for it in items {
                    self.frame_mut()[*var] = it;
                    match self.block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            StmtKind::Write { value, newline } => {
                let v = self.expr(value)?;
                let _ = write!(self.out, "{}", format_value(&v, &value.ty));
                if *newline {
                    let _ = writeln!(self.out);
                }
            }
            StmtKind::If { arms, otherwise } => {
                for (c, b) in arms {
                    if self.cond(c)? {
                        return self.block(b);
                    }
                }
                if let Some(b) = otherwise {
                    return self.block(b);
                }
            }
            StmtKind::While { cond, body } => {
                while self.cond(cond)? {
                    match self.block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            StmtKind::Repeat { count, body } => {
                let n = match count {
                    Some(c) => match self.expr(c)? {
                        Value::Int(n) => Some(n),
                        _ => unreachable!(),
                    },
                    None => None,
                };
                let mut i = 0i128;
                while n.is_none_or(|n| i < n) {
                    match self.block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    i += 1;
                }
            }
            StmtKind::ForRange { var, from, to, body } => {
                let (Value::Int(a), Value::Int(b)) = (self.expr(from)?, self.expr(to)?) else { unreachable!() };
                let step = if a <= b { 1 } else { -1 };
                let mut i = a;
                loop {
                    self.frame_mut()[*var] = Value::Int(i);
                    match self.block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    if i == b {
                        break;
                    }
                    i += step;
                }
            }
            StmtKind::Error => unreachable!("programa com erro não chega aos back-ends"),
            StmtKind::Break => return Ok(Flow::Break),
            StmtKind::Continue => return Ok(Flow::Continue),
            StmtKind::Return(v) => {
                let v = match v {
                    Some(e) => self.expr(e)?,
                    None => Value::Unit,
                };
                return Ok(Flow::Return(v));
            }
            StmtKind::Expr(e) => {
                self.expr(e)?;
            }
            StmtKind::Match { scrutinee, arms, otherwise } => {
                let Value::Variant(tag, fields) = self.expr(scrutinee)? else { unreachable!() };
                for arm in arms {
                    if arm.tags.contains(&tag) {
                        for (id, i) in &arm.binds {
                            self.frame_mut()[*id] = fields[*i].clone();
                        }
                        return self.block(&arm.body);
                    }
                }
                if let Some(b) = otherwise {
                    return self.block(b);
                }
            }
            StmtKind::Desist(m) => {
                let Value::Text(msg) = self.expr(m)? else { unreachable!() };
                return Err(Fault { code: None, message: msg, span: m.span, desist: true });
            }
        }
        Ok(Flow::Normal)
    }

    fn cond(&mut self, c: &Cond) -> Exec<bool> {
        Ok(match c {
            Cond::And(v) => {
                for c in v {
                    if !self.cond(c)? {
                        return Ok(false);
                    }
                }
                true
            }
            Cond::Or(v) => {
                for c in v {
                    if self.cond(c)? {
                        return Ok(true);
                    }
                }
                false
            }
            Cond::Not(c) => !self.cond(c)?,
            Cond::Cmp { rel, lhs, rhs } => {
                let a = self.expr(lhs)?;
                let b = self.expr(rhs)?;
                compare(*rel, &a, &b)
            }
            Cond::Test(e) => self.expr(e)? == Value::Bool(true),
            Cond::IsSome(e) => self.expr(e)? != Value::Nothing,
            Cond::IsVariant(e, tag) => matches!(self.expr(e)?, Value::Variant(t, _) if t == *tag),
        })
    }

    fn read_line(&mut self, span: Span) -> Exec<String> {
        let _ = self.out.flush();
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => return fault(L0912, textfmt::INPUT_ENDED, span),
            Ok(_) => {}
        }
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if self.echo {
            let _ = writeln!(self.out, "{line}");
        }
        Ok(line)
    }

    fn input(&mut self, ty: &Ty, span: Span) -> Exec<Value> {
        let line = self.read_line(span)?;
        parse_input(&line, ty).ok_or_else(|| Fault {
            code: Some(L0913),
            message: textfmt::bad_input(&ty.describe(), &line),
            span,
            desist: false,
        })
    }

    fn expr(&mut self, e: &hir::Expr) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(v) => Value::Int(*v),
            ExprKind::Real(v) => Value::Real(*v),
            ExprKind::Text(s) => Value::Text(s.clone()),
            ExprKind::Char(c) => Value::Char(*c),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Read(p) => {
                if p.proj.is_empty() {
                    self.get(p.root)
                } else {
                    self.read_place(p)?
                }
            }
            ExprKind::Input => self.input(&e.ty, e.span)?,
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l)?;
                let b = self.expr(r)?;
                arith(*op, a, b, &e.ty, e.span)?
            }
            ExprKind::Concat(parts) => {
                let mut s = String::new();
                for p in parts {
                    s.push_str(&format_value(&self.expr(p)?, &p.ty));
                }
                Value::Text(s)
            }
            ExprKind::Call { func, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.expr(&a.value)?);
                }
                let (ret, callee) = self.call(*func, vals, e.span)?;
                let ret = match ret {
                    Ok(v) => v,
                    // A falha só chega aqui dentro de um "Try", que a trata.
                    Err(f) => {
                        self.copy_back(*func, args, &callee)?;
                        return Err(f);
                    }
                };
                self.copy_back(*func, args, &callee)?;
                ret
            }
            ExprKind::Fact(c) => Value::Bool(self.cond(c)?),
            ExprKind::Unary(op, a) => match (op, &a.kind) {
                // O tamanho de um lugar se mede no próprio lugar: ler o valor inteiro
                // copiaria a coleção, e a conta ficaria proporcional ao tamanho dela.
                (UnOp::Len, ExprKind::Read(p)) => {
                    let steps = self.path(p)?;
                    Value::Int(self.len_at(p.root, &steps) as i128)
                }
                _ => {
                    let v = self.expr(a)?;
                    unary(*op, v, &e.ty, e.span)?
                }
            },
            ExprKind::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.expr(it)?);
                }
                Value::List(v)
            }
            ExprKind::Record(fields) => {
                let mut v = Vec::with_capacity(fields.len());
                for f in fields {
                    v.push(self.expr(f)?);
                }
                Value::Record(v)
            }
            ExprKind::Copy(a) | ExprKind::Wrap(a) | ExprKind::Unwrap(a) => self.expr(a)?,
            ExprKind::Nothing => Value::Nothing,
            ExprKind::Convert(a) => {
                let v = self.expr(a)?;
                convert(v, &a.ty, &e.ty, e.span)?
            }
            ExprKind::Variant { tag, fields } => {
                let mut v = Vec::with_capacity(fields.len());
                for f in fields {
                    v.push(self.expr(f)?);
                }
                Value::Variant(*tag, v)
            }
            ExprKind::VariantField { value, field } => match self.expr(value)? {
                Value::Variant(_, f) => f[*field].clone(),
                _ => unreachable!(),
            },
            ExprKind::Try { call, fallback } => match self.expr(call) {
                Ok(v) => v,
                Err(f) if f.desist => match fallback {
                    Some(fb) => self.expr(fb)?,
                    None => return Err(f),
                },
                Err(f) => return Err(f),
            },
            ExprKind::Take { place, index } => {
                let i = match index {
                    Some(ix) => match self.expr(ix)? {
                        Value::Int(n) => Some((n, ix.span)),
                        _ => unreachable!(),
                    },
                    None => None,
                };
                let steps = self.path(place)?;
                let Value::List(list) = self.place_mut(place.root, &steps) else { unreachable!() };
                match i {
                    None => match list.pop() {
                        Some(v) => v,
                        None => return fault(L0911, textfmt::EMPTY_TAKE, e.span),
                    },
                    Some((n, span)) => {
                        if n < 1 || n as usize > list.len() {
                            let len = list.len();
                            return fault(L0909, position_error(n, len), span);
                        }
                        list.remove(n as usize - 1)
                    }
                }
            }
        })
    }
}

fn walk<'a>(mut v: &'a Value, steps: &[Step]) -> &'a Value {
    for s in steps {
        v = match (v, s) {
            (Value::Record(f), Step::Field(i)) => &f[*i],
            (Value::List(l), Step::Index(i)) => &l[*i],
            _ => return v,
        };
    }
    v
}

fn walk_mut<'a>(mut v: &'a mut Value, steps: &[Step]) -> &'a mut Value {
    for s in steps {
        v = match (v, s) {
            (Value::Record(f), Step::Field(i)) => &mut f[*i],
            (Value::List(l), Step::Index(i)) => &mut l[*i],
            _ => unreachable!(),
        };
    }
    v
}

use textfmt::position_error;

fn unary(op: UnOp, v: Value, ty: &Ty, span: Span) -> Exec<Value> {
    Ok(match (op, v) {
        (UnOp::Len, Value::List(l)) => Value::Int(l.len() as i128),
        (UnOp::Len, Value::Text(t)) => Value::Int(t.chars().count() as i128),
        (UnOp::Neg, Value::Int(x)) => Value::Int(int_arith(BinOp::Sub, 0, x, int_ty(ty), span)?),
        (UnOp::Neg, Value::Real(x)) => Value::Real(-x),
        (UnOp::Abs, Value::Int(x)) => {
            let r = x.abs();
            if !int_ty(ty).contains(r) {
                return fault(L0902, textfmt::abs_overflow(x, int_ty(ty).bits, int_ty(ty).signed), span);
            }
            Value::Int(r)
        }
        (UnOp::Abs, Value::Real(x)) => Value::Real(x.abs()),
        (UnOp::Sqrt, Value::Real(x)) => {
            if x < 0.0 {
                return fault(L0907, textfmt::negative_sqrt(x), span);
            }
            Value::Real(x.sqrt())
        }
        (UnOp::BitNot, Value::Int(x)) => {
            let it = int_ty(ty);
            Value::Int(wrap(!x, it))
        }
        (op, v) => panic!("operação mal tipada: {op:?} {v:?}"),
    })
}

fn convert(v: Value, from: &Ty, to: &Ty, span: Span) -> Exec<Value> {
    Ok(match (v, to) {
        (v, Ty::Text) => Value::Text(format_value(&v, from)),
        (Value::Int(x), Ty::Int(it)) => {
            if !it.contains(x) {
                return fault(L0908, textfmt::convert_error(&x.to_string(), it.bits, it.signed), span);
            }
            Value::Int(x)
        }
        (Value::Int(x), Ty::Real(b)) => Value::Real(if *b == 32 { x as f64 as f32 as f64 } else { x as f64 }),
        (Value::Real(x), Ty::Int(it)) => {
            if !textfmt::real_fits(x, it.bits, it.signed) {
                return fault(L0908, textfmt::convert_error(&format_real(x), it.bits, it.signed), span);
            }
            Value::Int(x.trunc() as i128)
        }
        (Value::Real(x), Ty::Real(b)) => Value::Real(if *b == 32 { x as f32 as f64 } else { x }),
        (v, _) => v,
    })
}

fn int_ty(ty: &Ty) -> IntTy {
    match ty {
        Ty::Int(i) => *i,
        _ => IntTy::I64,
    }
}

/// Reinterpreta os bits baixos de `x` na largura do tipo (para operações de bits).
fn wrap(x: i128, it: IntTy) -> i128 {
    let mask = (1i128 << it.bits) - 1;
    let low = x & mask;
    if it.signed && low >> (it.bits - 1) & 1 == 1 { low - (1i128 << it.bits) } else { low }
}

pub fn parse_input(line: &str, ty: &Ty) -> Option<Value> {
    match ty {
        Ty::Int(it) => textfmt::parse_int(line, it.bits, it.signed).map(Value::Int),
        Ty::Real(bits) => textfmt::parse_real(line).map(|v| Value::Real(if *bits == 32 { v as f32 as f64 } else { v })),
        Ty::Bool => textfmt::parse_bool(line).map(Value::Bool),
        Ty::Char => textfmt::parse_char(line).map(Value::Char),
        Ty::Text => Some(Value::Text(line.to_string())),
        _ => None,
    }
}

pub fn compare(rel: Rel, a: &Value, b: &Value) -> bool {
    let ord =
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
            (Value::Real(x), Value::Real(y)) => x.partial_cmp(y),
            (Value::Char(x), Value::Char(y)) => x.partial_cmp(y),
            (Value::Text(x), Value::Text(y)) => x.partial_cmp(y),
            (Value::Bool(x), Value::Bool(y)) => x.partial_cmp(y),
            // Coleções e registros só se comparam por igualdade.
            (Value::Nothing, Value::Nothing) => Some(Ordering::Equal),
            (Value::Nothing, _) | (_, Value::Nothing) => Some(Ordering::Less),
            (Value::List(_), Value::List(_)) | (Value::Record(_), Value::Record(_)) | (Value::Variant(..), Value::Variant(..)) => {
                if a == b { Some(Ordering::Equal) } else { Some(Ordering::Less) }
            }
            _ => None,
        };
    let Some(ord) = ord else { return rel == Rel::Different };
    match rel {
        Rel::Greater => ord == Ordering::Greater,
        Rel::Less => ord == Ordering::Less,
        Rel::Equal => ord == Ordering::Equal,
        Rel::Different => ord != Ordering::Equal,
        Rel::AtLeast => ord != Ordering::Less,
        Rel::AtMost => ord != Ordering::Greater,
    }
}

fn fault<T>(code: Code, message: impl Into<String>, span: Span) -> Exec<T> {
    Err(Fault { code: Some(code), message: message.into(), span, desist: false })
}

pub fn arith(op: BinOp, a: Value, b: Value, ty: &Ty, span: Span) -> Exec<Value> {
    match (a, b, ty) {
        (Value::Int(x), Value::Int(y), Ty::Int(it)) => int_arith(op, x, y, *it, span).map(Value::Int),
        (Value::Real(x), Value::Real(y), Ty::Real(bits)) => {
            let r = match op {
                BinOp::Add => x + y,
                BinOp::Sub => x - y,
                BinOp::Mul => x * y,
                BinOp::Div => x / y,
                BinOp::Rem => x % y,
                BinOp::Pow => x.powf(y),
                _ => unreachable!(),
            };
            Ok(Value::Real(if *bits == 32 { r as f32 as f64 } else { r }))
        }
        (a, b, _) => panic!("aritmética mal tipada: {a:?} {b:?}"),
    }
}

pub fn int_arith(op: BinOp, x: i128, y: i128, it: IntTy, span: Span) -> Exec<i128> {
    let r = match op {
        BinOp::Add => x + y,
        BinOp::Sub => x - y,
        BinOp::Mul => x.checked_mul(y).unwrap_or(i128::MAX),
        BinOp::Div | BinOp::Rem => {
            if y == 0 {
                return if op == BinOp::Div { fault(L0904, textfmt::DIV_ZERO, span) } else { fault(L0905, textfmt::REM_ZERO, span) };
            }
            if op == BinOp::Div { x / y } else { x % y }
        }
        BinOp::Pow => {
            if y < 0 {
                return fault(L0906, textfmt::NEG_POW, span);
            }
            let mut r: i128 = 1;
            for _ in 0..y.min(200) {
                r = r.checked_mul(x).filter(|r| it.contains(*r)).ok_or_else(|| Fault {
                    code: Some(L0901),
                    message: textfmt::pow_overflow(it.bits, it.signed),
                    span,
                    desist: false,
                })?;
            }
            r
        }
        BinOp::BitAnd => return Ok(wrap(x & y, it)),
        BinOp::BitOr => return Ok(wrap(x | y, it)),
        BinOp::BitXor => return Ok(wrap(x ^ y, it)),
        BinOp::Shl | BinOp::Shr => {
            if y < 0 || y >= it.bits as i128 {
                return fault(L0903, textfmt::shift_range(y, it.bits, it.signed), span);
            }
            let bits = x & ((1i128 << it.bits) - 1);
            return Ok(wrap(if op == BinOp::Shl { bits << y } else { bits >> y }, it));
        }
    };
    if !it.contains(r) {
        return fault(L0900, textfmt::overflow(it.bits, it.signed), span);
    }
    Ok(r)
}

/// Como um valor aparece na saída. Lista entre colchetes, separada por ponto e vírgula
/// (a vírgula já é a decimal); registro por extenso, como se constrói:
/// "uma conta com o saldo 10 e o titular Ana".
pub fn format_value(v: &Value, ty: &Ty) -> String {
    match (v, ty) {
        (Value::Int(i), _) => i.to_string(),
        (Value::Real(r), _) => format_real(*r),
        (Value::Bool(b), _) => textfmt::format_bool(*b).to_string(),
        (Value::Char(c), _) => c.to_string(),
        (Value::Text(s), _) => s.clone(),
        (Value::List(items), Ty::List(elem)) => {
            let parts: Vec<String> = items.iter().map(|x| format_value(x, elem)).collect();
            format!("[{}]", parts.join("; "))
        }
        (Value::Record(fields), Ty::Record(r)) => {
            let parts: Vec<String> = fields
                .iter()
                .zip(&r.fields)
                .map(|(v, (name, t))| format!("{} {name} {}", field_article(name, r), format_value(v, t)))
                .collect();
            let joined = textfmt::join_e(&parts);
            format!("{} {} com {joined}", if r.feminine { "uma" } else { "um" }, r.name)
        }
        (Value::Unit, _) => String::new(),
        (Value::Nothing, _) => "nada".to_string(),
        (Value::Variant(tag, fields), Ty::Enum(e)) => {
            let v = &e.variants[*tag];
            let head = format!("{} {}", if v.feminine { "uma" } else { "um" }, v.name);
            if fields.is_empty() {
                return head;
            }
            let parts: Vec<String> = fields
                .iter()
                .zip(&v.fields)
                .zip(&v.field_articles)
                .map(|((x, (name, t)), (_, art))| format!("{art} {name} {}", format_value(x, t)))
                .collect();
            format!("{head} com {}", textfmt::join_e(&parts))
        }
        (v, Ty::Maybe(inner)) => format_value(v, inner),
        (v, t) => panic!("valor {v:?} com tipo {t:?}"),
    }
}

pub fn field_article(name: &str, r: &crate::types::RecordTy) -> &'static str {
    r.field_articles.iter().find(|(n, _)| n == name).map(|(_, a)| *a).unwrap_or("o")
}

pub use textfmt::format_real;
