//! Diagnósticos: categorias gramaticais e semânticas, e a renderização no estilo do
//! rustc, em português, com o trecho do código sublinhado.

use crate::span::{SourceFile, Span};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Lexical,
    Morphology,
    Agreement,
    Regency,
    Syntax,
    Scope,
    Type,
    Mutability,
    Ownership,
    Borrow,
    Runtime,
    Warning,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Lexical => "Erro léxico",
            Category::Morphology => "Erro de morfologia",
            Category::Agreement => "Erro de concordância",
            Category::Regency => "Erro de regência",
            Category::Syntax => "Erro de sintaxe",
            Category::Scope => "Erro de escopo",
            Category::Type => "Erro de tipo",
            Category::Mutability => "Erro de mutabilidade",
            Category::Ownership => "Erro de posse",
            Category::Borrow => "Erro de empréstimo",
            Category::Runtime => "Erro de execução",
            Category::Warning => "Aviso",
        }
    }

    pub fn is_error(self) -> bool {
        self != Category::Warning
    }
}

/// Um rótulo secundário: aponta outro lugar do código com uma explicação curta.
#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub category: Category,
    pub message: String,
    pub span: Span,
    /// Explicação curta sob o sublinhado principal.
    pub primary: Option<String>,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(category: Category, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic {
            category,
            message: message.into(),
            span,
            primary: None,
            labels: Vec::new(),
            notes: Vec::new(),
            hint: None,
        }
    }

    pub fn primary(mut self, text: impl Into<String>) -> Self {
        self.primary = Some(text.into());
        self
    }

    pub fn label(mut self, span: Span, text: impl Into<String>) -> Self {
        self.labels.push(Label { span, text: text.into() });
        self
    }

    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.notes.push(text.into());
        self
    }

    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint = Some(text.into());
        self
    }

    pub fn maybe_hint(mut self, text: Option<String>) -> Self {
        if text.is_some() {
            self.hint = text;
        }
        self
    }
}

/// Resultado de uma etapa que pode falhar com um diagnóstico.
pub type Result<T> = std::result::Result<T, Box<Diagnostic>>;

pub fn err<T>(d: Diagnostic) -> Result<T> {
    Err(Box::new(d))
}

/// Cores ANSI, ligadas só quando a saída é um terminal.
#[derive(Clone, Copy)]
pub struct Style {
    pub color: bool,
}

impl Style {
    fn paint(&self, code: &str, s: &str) -> String {
        if self.color { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_string() }
    }
}

struct Mark {
    line: usize,
    col_start: usize,
    col_end: usize,
    primary: bool,
    text: Option<String>,
}

/// Renderiza o diagnóstico com o trecho do código e os sublinhados.
pub fn render(d: &Diagnostic, src: &SourceFile, style: Style) -> String {
    let mut out = String::new();
    let head_color = if d.category.is_error() { "1;31" } else { "1;33" };
    let _ = writeln!(
        out,
        "{}{}",
        style.paint(head_color, &format!("{}:", d.category.label())),
        style.paint("1", &format!(" {}", d.message))
    );
    let (line, col) = src.line_col(d.span.start);
    let mut marks = vec![mark(src, d.span, true, d.primary.clone())];
    for l in &d.labels {
        marks.push(mark(src, l.span, false, Some(l.text.clone())));
    }
    let max_line = marks.iter().map(|m| m.line).max().unwrap_or(line);
    let width = max_line.to_string().len();
    let pad = " ".repeat(width);
    let bar = style.paint("1;34", "|");
    let _ = writeln!(out, "{pad}{} {}:{}:{}", style.paint("1;34", "-->"), src.name(), line, col);
    let _ = writeln!(out, "{pad} {bar}");
    let mut lines: Vec<usize> = marks.iter().map(|m| m.line).collect();
    lines.sort_unstable();
    lines.dedup();
    let mut prev: Option<usize> = None;
    for l in lines {
        if let Some(p) = prev
            && l > p + 1
        {
            let _ = writeln!(out, "{}", style.paint("1;34", "..."));
        }
        prev = Some(l);
        let text = src.line_text(l).replace('\t', "    ");
        let num = format!("{l:>width$}");
        let _ = writeln!(out, "{} {bar} {}", style.paint("1;34", &num), text);
        let mut here: Vec<&Mark> = marks.iter().filter(|m| m.line == l).collect();
        here.sort_by_key(|m| m.col_start);
        for m in here {
            let len = m.col_end.saturating_sub(m.col_start).max(1);
            let ch = if m.primary { "^" } else { "-" };
            let color = if m.primary { head_color } else { "1;34" };
            let under = style.paint(color, &ch.repeat(len));
            let label = m.text.as_deref().map(|t| format!(" {}", style.paint(color, t))).unwrap_or_default();
            let _ = writeln!(out, "{pad} {bar} {}{}{}", " ".repeat(m.col_start - 1), under, label);
        }
    }
    for n in &d.notes {
        let _ = writeln!(out, "{pad} {} {}", style.paint("1;34", "="), n);
    }
    if let Some(h) = &d.hint {
        let _ = writeln!(out, "{pad} {} {}", style.paint("1;34", "="), style.paint("1", &format!("dica: {h}")));
    }
    out
}

fn mark(src: &SourceFile, span: Span, primary: bool, text: Option<String>) -> Mark {
    let (line, col_start) = src.line_col(span.start);
    // Sublinhado restrito à primeira linha do intervalo.
    let line_end_offset = {
        let t = &src.text[span.start as usize..];
        span.start as usize + t.find('\n').unwrap_or(t.len())
    };
    let end = (span.end as usize).min(line_end_offset).max(span.start as usize);
    let col_end = col_start + src.text[span.start as usize..end].chars().count();
    Mark { line, col_start, col_end, primary, text }
}
