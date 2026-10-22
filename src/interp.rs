//! Interpretador da HIR. É a referência de semântica: o nativo tem que produzir a mesma
//! saída, byte a byte.

use crate::ast::{BinOp, ParamMode, Rel, UnOp};
use crate::hir::{self, Cond, ExprKind, PlaceProj, Stmt, Var};
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
    Unit,
}

/// Um passo do caminho até um lugar, já com as posições calculadas.
enum Step {
    Field(usize),
    /// Posição a partir de 0.
    Index(usize),
}

/// Falha de execução: a mensagem e onde aconteceu.
#[derive(Debug)]
pub struct Fault {
    pub message: String,
    pub span: Span,
    /// Falha declarada ("Desisto com ..."), que sobe até quem a trata.
    pub desist: bool,
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

pub struct Machine<'p, W: Write, R: BufRead> {
    prog: &'p hir::Program,
    out: W,
    input: R,
    globals: Vec<Value>,
    depth: usize,
    /// Ecoa a entrada lida na saída (útil quando a entrada vem de arquivo).
    pub echo: bool,
}

impl<'p, W: Write, R: BufRead> Machine<'p, W, R> {
    pub fn new(prog: &'p hir::Program, out: W, input: R) -> Self {
        Machine { prog, out, input, globals: vec![Value::Unit; prog.globals.len()], depth: 0, echo: false }
    }

    pub fn run(&mut self) -> Exec<()> {
        let mut frame = vec![Value::Unit; self.prog.init.locals.len()];
        let r = self.block(&self.prog.init.body, &mut frame).and_then(|_| self.call(self.prog.main, Vec::new(), Span::default()));
        let _ = self.out.flush();
        match r {
            Ok((Err(f), _)) => Err(f),
            Ok(_) => Ok(()),
            Err(f) => Err(f),
        }
    }

    /// Chama a função. O resultado de dentro é o valor devolvido ou a falha declarada;
    /// o quadro volta junto para os parâmetros confiados.
    fn call(&mut self, func: hir::FuncId, args: Vec<Value>, span: Span) -> Exec<(Result<Value, Fault>, Vec<Value>)> {
        let f = &self.prog.funcs[func];
        if self.depth >= MAX_DEPTH {
            return fault(textfmt::stack_overflow(), span);
        }
        self.depth += 1;
        let mut frame = vec![Value::Unit; f.locals.len()];
        for ((id, _), v) in f.params.iter().zip(args) {
            frame[*id] = v;
        }
        let r = self.block(&f.body, &mut frame);
        self.depth -= 1;
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
    fn copy_back(&mut self, func: hir::FuncId, args: &[hir::Arg], callee: &[Value], frame: &mut [Value]) -> Exec<()> {
        let f = &self.prog.funcs[func];
        for (a, (id, mode)) in args.iter().zip(&f.params) {
            if *mode == ParamMode::Lent
                && let Some(p) = &a.place
            {
                self.write_place(p, callee[*id].clone(), frame)?;
            }
        }
        Ok(())
    }

    fn block(&mut self, b: &hir::Block, frame: &mut [Value]) -> Exec<Flow> {
        for s in &b.0 {
            match self.stmt(s, frame)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn get(&self, v: Var, frame: &[Value]) -> Value {
        match v {
            Var::Local(id) => frame[id].clone(),
            Var::Global(id) => self.globals[id].clone(),
        }
    }

    fn set(&mut self, v: Var, value: Value, frame: &mut [Value]) {
        match v {
            Var::Local(id) => frame[id] = value,
            Var::Global(id) => self.globals[id] = value,
        }
    }

    /// Calcula o caminho de um lugar, conferindo cada posição contra o tamanho da coleção.
    fn path(&mut self, p: &hir::Place, frame: &mut [Value]) -> Exec<Vec<Step>> {
        let mut steps = Vec::new();
        for pr in &p.proj {
            match pr {
                PlaceProj::Field(i) => steps.push(Step::Field(*i)),
                PlaceProj::Index(e, span) => {
                    let Value::Int(n) = self.expr(e, frame)? else { unreachable!() };
                    let len = self.len_at(p.root, &steps, frame);
                    if n < 1 || n as usize > len {
                        return fault(position_error(n, len), *span);
                    }
                    steps.push(Step::Index(n as usize - 1));
                }
                PlaceProj::Last(span) => {
                    let len = self.len_at(p.root, &steps, frame);
                    if len == 0 {
                        return fault(textfmt::EMPTY_LAST, *span);
                    }
                    steps.push(Step::Index(len - 1));
                }
            }
        }
        Ok(steps)
    }

    fn len_at(&self, root: Var, steps: &[Step], frame: &[Value]) -> usize {
        let base = match root {
            Var::Local(id) => &frame[id],
            Var::Global(id) => &self.globals[id],
        };
        match walk(base, steps) {
            Value::List(v) => v.len(),
            Value::Text(t) => t.chars().count(),
            _ => 0,
        }
    }

    fn read_place(&mut self, p: &hir::Place, frame: &mut [Value]) -> Exec<Value> {
        let steps = self.path(p, frame)?;
        let base = match p.root {
            Var::Local(id) => &frame[id],
            Var::Global(id) => &self.globals[id],
        };
        // A posição num texto é a de um caractere.
        if let (Some(Step::Index(i)), Value::Text(t)) = (steps.last(), walk(base, &steps[..steps.len().saturating_sub(1)])) {
            return Ok(Value::Char(t.chars().nth(*i).unwrap()));
        }
        Ok(walk(base, &steps).clone())
    }

    fn place_mut<'a>(&'a mut self, root: Var, steps: &[Step], frame: &'a mut [Value]) -> &'a mut Value {
        let base = match root {
            Var::Local(id) => &mut frame[id],
            Var::Global(id) => &mut self.globals[id],
        };
        walk_mut(base, steps)
    }

    fn write_place(&mut self, p: &hir::Place, v: Value, frame: &mut [Value]) -> Exec<()> {
        let steps = self.path(p, frame)?;
        *self.place_mut(p.root, &steps, frame) = v;
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt, frame: &mut [Value]) -> Exec<Flow> {
        match s {
            Stmt::Let { var, value } => {
                let v = self.expr(value, frame)?;
                self.set(*var, v, frame);
            }
            Stmt::Assign { place, value } => {
                let v = self.expr(value, frame)?;
                self.write_place(place, v, frame)?;
            }
            Stmt::Compound { place, op, value, span } => {
                let cur = self.read_place(place, frame)?;
                let v = self.expr(value, frame)?;
                let r = arith(*op, cur, v, &place.ty, *span)?;
                self.write_place(place, r, frame)?;
            }
            Stmt::Push { place, value } => {
                let v = self.expr(value, frame)?;
                let steps = self.path(place, frame)?;
                let Value::List(list) = self.place_mut(place.root, &steps, frame) else { unreachable!() };
                list.push(v);
            }
            Stmt::ForEach { var, source, body } => {
                let items: Vec<Value> = match self.expr(source, frame)? {
                    Value::List(v) => v,
                    Value::Text(t) => t.chars().map(Value::Char).collect(),
                    _ => unreachable!(),
                };
                for it in items {
                    frame[*var] = it;
                    match self.block(body, frame)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            Stmt::Write { value, newline } => {
                let v = self.expr(value, frame)?;
                let _ = write!(self.out, "{}", format_value(&v, &value.ty));
                if *newline {
                    let _ = writeln!(self.out);
                }
            }
            Stmt::If { arms, otherwise } => {
                for (c, b) in arms {
                    if self.cond(c, frame)? {
                        return self.block(b, frame);
                    }
                }
                if let Some(b) = otherwise {
                    return self.block(b, frame);
                }
            }
            Stmt::While { cond, body } => {
                while self.cond(cond, frame)? {
                    match self.block(body, frame)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            Stmt::Repeat { count, body } => {
                let n = match count {
                    Some(c) => match self.expr(c, frame)? {
                        Value::Int(n) => Some(n),
                        _ => unreachable!(),
                    },
                    None => None,
                };
                let mut i = 0i128;
                while n.is_none_or(|n| i < n) {
                    match self.block(body, frame)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    i += 1;
                }
            }
            Stmt::ForRange { var, from, to, body } => {
                let (Value::Int(a), Value::Int(b)) = (self.expr(from, frame)?, self.expr(to, frame)?) else { unreachable!() };
                let step = if a <= b { 1 } else { -1 };
                let mut i = a;
                loop {
                    frame[*var] = Value::Int(i);
                    match self.block(body, frame)? {
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
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Return(v) => {
                let v = match v {
                    Some(e) => self.expr(e, frame)?,
                    None => Value::Unit,
                };
                return Ok(Flow::Return(v));
            }
            Stmt::Expr(e) => {
                self.expr(e, frame)?;
            }
            Stmt::Desist(m) => {
                let Value::Text(msg) = self.expr(m, frame)? else { unreachable!() };
                return Err(Fault { message: msg, span: m.span, desist: true });
            }
        }
        Ok(Flow::Normal)
    }

    fn cond(&mut self, c: &Cond, frame: &mut [Value]) -> Exec<bool> {
        Ok(match c {
            Cond::And(v) => {
                for c in v {
                    if !self.cond(c, frame)? {
                        return Ok(false);
                    }
                }
                true
            }
            Cond::Or(v) => {
                for c in v {
                    if self.cond(c, frame)? {
                        return Ok(true);
                    }
                }
                false
            }
            Cond::Not(c) => !self.cond(c, frame)?,
            Cond::Cmp { rel, lhs, rhs } => {
                let a = self.expr(lhs, frame)?;
                let b = self.expr(rhs, frame)?;
                compare(*rel, &a, &b)
            }
            Cond::Test(e) => self.expr(e, frame)? == Value::Bool(true),
            Cond::IsSome(e) => self.expr(e, frame)? != Value::Nothing,
        })
    }

    fn read_line(&mut self, span: Span) -> Exec<String> {
        let _ = self.out.flush();
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => return fault(textfmt::INPUT_ENDED, span),
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
        parse_input(&line, ty).ok_or_else(|| Fault { message: textfmt::bad_input(&ty.describe(), &line), span, desist: false })
    }

    fn expr(&mut self, e: &hir::Expr, frame: &mut [Value]) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(v) => Value::Int(*v),
            ExprKind::Real(v) => Value::Real(*v),
            ExprKind::Text(s) => Value::Text(s.clone()),
            ExprKind::Char(c) => Value::Char(*c),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Read(p) => {
                if p.proj.is_empty() {
                    self.get(p.root, frame)
                } else {
                    self.read_place(p, frame)?
                }
            }
            ExprKind::Input => self.input(&e.ty, e.span)?,
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l, frame)?;
                let b = self.expr(r, frame)?;
                arith(*op, a, b, &e.ty, e.span)?
            }
            ExprKind::Concat(parts) => {
                let mut s = String::new();
                for p in parts {
                    s.push_str(&format_value(&self.expr(p, frame)?, &p.ty));
                }
                Value::Text(s)
            }
            ExprKind::Call { func, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.expr(&a.value, frame)?);
                }
                let (ret, callee) = self.call(*func, vals, e.span)?;
                let ret = match ret {
                    Ok(v) => v,
                    // A falha só chega aqui dentro de um "Try", que a trata.
                    Err(f) => {
                        self.copy_back(*func, args, &callee, frame)?;
                        return Err(f);
                    }
                };
                self.copy_back(*func, args, &callee, frame)?;
                ret
            }
            ExprKind::Fact(c) => Value::Bool(self.cond(c, frame)?),
            ExprKind::Unary(op, a) => {
                let v = self.expr(a, frame)?;
                unary(*op, v, &e.ty, e.span)?
            }
            ExprKind::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.expr(it, frame)?);
                }
                Value::List(v)
            }
            ExprKind::Record(fields) => {
                let mut v = Vec::with_capacity(fields.len());
                for f in fields {
                    v.push(self.expr(f, frame)?);
                }
                Value::Record(v)
            }
            ExprKind::Copy(a) | ExprKind::Wrap(a) | ExprKind::Unwrap(a) => self.expr(a, frame)?,
            ExprKind::Nothing => Value::Nothing,
            ExprKind::Try { call, fallback } => match self.expr(call, frame) {
                Ok(v) => v,
                Err(f) if f.desist => match fallback {
                    Some(fb) => self.expr(fb, frame)?,
                    None => return Err(f),
                },
                Err(f) => return Err(f),
            },
            ExprKind::Take { place, index } => {
                let i = match index {
                    Some(ix) => match self.expr(ix, frame)? {
                        Value::Int(n) => Some((n, ix.span)),
                        _ => unreachable!(),
                    },
                    None => None,
                };
                let steps = self.path(place, frame)?;
                let Value::List(list) = self.place_mut(place.root, &steps, frame) else { unreachable!() };
                match i {
                    None => match list.pop() {
                        Some(v) => v,
                        None => return fault(textfmt::EMPTY_TAKE, e.span),
                    },
                    Some((n, span)) => {
                        if n < 1 || n as usize > list.len() {
                            let len = list.len();
                            return fault(position_error(n, len), span);
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
                return fault(textfmt::abs_overflow(x, int_ty(ty).bits, int_ty(ty).signed), span);
            }
            Value::Int(r)
        }
        (UnOp::Abs, Value::Real(x)) => Value::Real(x.abs()),
        (UnOp::Sqrt, Value::Real(x)) => {
            if x < 0.0 {
                return fault(textfmt::negative_sqrt(x), span);
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
        Ty::Real(_) => textfmt::parse_real(line).map(Value::Real),
        Ty::Bool => textfmt::parse_bool(line).map(Value::Bool),
        Ty::Char => textfmt::parse_char(line).map(Value::Char),
        Ty::Text => Some(Value::Text(line.to_string())),
        _ => None,
    }
}

pub fn compare(rel: Rel, a: &Value, b: &Value) -> bool {
    let ord = match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
        (Value::Real(x), Value::Real(y)) => x.partial_cmp(y),
        (Value::Char(x), Value::Char(y)) => x.partial_cmp(y),
        (Value::Text(x), Value::Text(y)) => x.partial_cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.partial_cmp(y),
        // Coleções e registros só se comparam por igualdade.
        (Value::Nothing, Value::Nothing) => Some(Ordering::Equal),
        (Value::Nothing, _) | (_, Value::Nothing) => Some(Ordering::Less),
        (Value::List(_), Value::List(_)) | (Value::Record(_), Value::Record(_)) => {
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

fn fault<T>(message: impl Into<String>, span: Span) -> Exec<T> {
    Err(Fault { message: message.into(), span, desist: false })
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
                return fault(if op == BinOp::Div { textfmt::DIV_ZERO } else { textfmt::REM_ZERO }, span);
            }
            if op == BinOp::Div { x / y } else { x % y }
        }
        BinOp::Pow => {
            if y < 0 {
                return fault(textfmt::NEG_POW, span);
            }
            let mut r: i128 = 1;
            for _ in 0..y.min(200) {
                r = r.checked_mul(x).filter(|r| it.contains(*r)).ok_or_else(|| Fault {
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
                return fault(textfmt::shift_range(y, it.bits, it.signed), span);
            }
            let bits = x & ((1i128 << it.bits) - 1);
            return Ok(wrap(if op == BinOp::Shl { bits << y } else { bits >> y }, it));
        }
    };
    if !it.contains(r) {
        return fault(textfmt::overflow(it.bits, it.signed), span);
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
        (v, Ty::Maybe(inner)) => format_value(v, inner),
        (v, t) => panic!("valor {v:?} com tipo {t:?}"),
    }
}

fn field_article(name: &str, r: &crate::types::RecordTy) -> &'static str {
    r.field_articles.iter().find(|(n, _)| n == name).map(|(_, a)| *a).unwrap_or("o")
}

pub use textfmt::format_real;
