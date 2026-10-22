//! Interpretador da HIR. É a referência de semântica: o nativo tem que produzir a mesma
//! saída, byte a byte.

use crate::ast::BinOp;
use crate::ast::Rel;
use crate::hir::{self, Cond, ExprKind, Stmt};
use crate::span::Span;
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
    Unit,
}

/// Falha de execução: a mensagem e onde aconteceu.
#[derive(Debug)]
pub struct Fault {
    pub message: String,
    pub span: Span,
}

type Exec<T> = std::result::Result<T, Fault>;

/// Como um bloco terminou.
enum Flow {
    Normal,
    Break,
    Continue,
}

pub struct Machine<'p, W: Write, R: BufRead> {
    prog: &'p hir::Program,
    out: W,
    input: R,
    /// Ecoa a entrada lida na saída (útil quando a entrada vem de arquivo).
    pub echo: bool,
}

impl<'p, W: Write, R: BufRead> Machine<'p, W, R> {
    pub fn new(prog: &'p hir::Program, out: W, input: R) -> Self {
        Machine { prog, out, input, echo: false }
    }

    pub fn run(&mut self) -> Exec<()> {
        let f = &self.prog.funcs[self.prog.main];
        let mut frame = vec![Value::Unit; f.locals.len()];
        let r = self.block(&f.body, &mut frame);
        let _ = self.out.flush();
        r.map(|_| ())
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

    fn stmt(&mut self, s: &Stmt, frame: &mut [Value]) -> Exec<Flow> {
        match s {
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
                    if let Flow::Break = self.block(body, frame)? {
                        break;
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
                    if let Flow::Break = self.block(body, frame)? {
                        break;
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
                    if let Flow::Break = self.block(body, frame)? {
                        break;
                    }
                    if i == b {
                        break;
                    }
                    i += step;
                }
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Let { local, value } => {
                frame[*local] = self.expr(value, frame)?;
            }
            Stmt::Assign { place, value } => {
                frame[place.root] = self.expr(value, frame)?;
            }
            Stmt::Compound { place, op, value, span } => {
                let cur = frame[place.root].clone();
                let v = self.expr(value, frame)?;
                frame[place.root] = arith(*op, cur, v, &place.ty, *span)?;
            }
            Stmt::Write { value, newline } => {
                let v = self.expr(value, frame)?;
                let _ = write!(self.out, "{}", format_value(&v));
                if *newline {
                    let _ = writeln!(self.out);
                }
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
        })
    }

    fn read_line(&mut self, span: Span) -> Exec<String> {
        let _ = self.out.flush();
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => return fault("A entrada acabou antes da hora: o programa esperava ler mais.", span),
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
        parse_input(&line, ty).ok_or_else(|| Fault { message: format!("Esperava {}, mas li \"{line}\".", ty.describe()), span })
    }

    fn expr(&mut self, e: &hir::Expr, frame: &mut [Value]) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(v) => Value::Int(*v),
            ExprKind::Real(v) => Value::Real(*v),
            ExprKind::Text(s) => Value::Text(s.clone()),
            ExprKind::Char(c) => Value::Char(*c),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Read(p) => frame[p.root].clone(),
            ExprKind::Input => self.input(&e.ty, e.span)?,
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l, frame)?;
                let b = self.expr(r, frame)?;
                arith(*op, a, b, &e.ty, e.span)?
            }
            ExprKind::Concat(parts) => {
                let mut s = String::new();
                for p in parts {
                    s.push_str(&format_value(&self.expr(p, frame)?));
                }
                Value::Text(s)
            }
        })
    }
}

pub fn parse_input(line: &str, ty: &Ty) -> Option<Value> {
    let t = line.trim();
    match ty {
        Ty::Int(it) => t.parse::<i128>().ok().filter(|v| it.contains(*v)).map(Value::Int),
        Ty::Real(_) => t.replace(',', ".").parse::<f64>().ok().map(Value::Real),
        Ty::Bool => match t.to_lowercase().as_str() {
            "verdadeiro" | "v" | "sim" | "s" => Some(Value::Bool(true)),
            "falso" | "f" | "não" | "nao" | "n" => Some(Value::Bool(false)),
            _ => None,
        },
        Ty::Char => {
            let mut c = t.chars();
            let first = c.next()?;
            c.next().is_none().then_some(Value::Char(first))
        }
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
    Err(Fault { message: message.into(), span })
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
                return fault(if op == BinOp::Div { "Divisão por zero." } else { "Resto de divisão por zero." }, span);
            }
            if op == BinOp::Div { x / y } else { x % y }
        }
        _ => unreachable!(),
    };
    if !it.contains(r) {
        return fault(format!("Estouro: o resultado não cabe num {} (vai de {} a {}).", it.noun(false), it.min(), it.max()), span);
    }
    Ok(r)
}

pub fn format_value(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Real(r) => format_real(*r),
        Value::Bool(b) => if *b { "verdadeiro" } else { "falso" }.to_string(),
        Value::Char(c) => c.to_string(),
        Value::Text(s) => s.clone(),
        Value::Unit => String::new(),
    }
}

/// Real com vírgula decimal, arredondado a 12 algarismos significativos.
pub fn format_real(v: f64) -> String {
    if v.is_nan() {
        return "indefinido".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "infinito" } else { "-infinito" }.into();
    }
    let rounded: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    let s = if rounded != 0.0 && (rounded.abs() >= 1e15 || rounded.abs() < 1e-6) {
        let e = format!("{rounded:e}");
        e
    } else {
        format!("{rounded}")
    };
    let s = if s == "-0" { "0".to_string() } else { s };
    s.replace('.', ",")
}
