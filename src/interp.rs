//! Interpretador da HIR. É a referência de semântica: o nativo tem que produzir a mesma
//! saída, byte a byte.

use crate::ast::BinOp;
use crate::hir::{self, ExprKind, Stmt};
use crate::span::Span;
use crate::types::{IntTy, Ty};
use std::io::Write;

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

pub struct Machine<'p, W: Write> {
    prog: &'p hir::Program,
    out: W,
}

impl<'p, W: Write> Machine<'p, W> {
    pub fn new(prog: &'p hir::Program, out: W) -> Self {
        Machine { prog, out }
    }

    pub fn run(&mut self) -> Exec<()> {
        let f = &self.prog.funcs[self.prog.main];
        let mut frame = vec![Value::Unit; f.locals.len()];
        self.block(&f.body, &mut frame)?;
        let _ = self.out.flush();
        Ok(())
    }

    fn block(&mut self, b: &hir::Block, frame: &mut [Value]) -> Exec<()> {
        for s in &b.0 {
            self.stmt(s, frame)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt, frame: &mut [Value]) -> Exec<()> {
        match s {
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
        Ok(())
    }

    fn expr(&mut self, e: &hir::Expr, frame: &mut [Value]) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(v) => Value::Int(*v),
            ExprKind::Real(v) => Value::Real(*v),
            ExprKind::Text(s) => Value::Text(s.clone()),
            ExprKind::Char(c) => Value::Char(*c),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Read(p) => frame[p.root].clone(),
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
