//! O interpretador da Lavra no navegador. O site chama `check` enquanto se escreve, para
//! sublinhar os erros, e `run` para rodar, dentro de um Web Worker: um laço sem fim não
//! trava a página, e o botão de parar encerra o worker. A saída vai saindo aos pedaços.

use lavra::diagnostics::{Diagnostic, Style, render};
use lavra::span::SourceFile;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

const NAME: &str = "programa.lav";

/// Quantas chamadas aninhadas cabem no navegador. A pilha do motor JavaScript (cerca de 1 MiB
/// no V8) acaba por volta de 1 500 chamadas da Lavra; o limite fica abaixo, para a falha ser
/// a da língua ("Estouro de pilha") e não a do motor.
const DEPTH: usize = 1000;

/// A posição de um byte do texto em unidades UTF-16, como o JavaScript conta.
fn utf16(source: &str, byte: u32) -> usize {
    let byte = (byte as usize).min(source.len());
    let byte = (0..=byte).rev().find(|&b| source.is_char_boundary(b)).unwrap_or(0);
    source[..byte].encode_utf16().count()
}

fn shown(diags: &[Diagnostic], src: &SourceFile) -> String {
    diags.iter().map(|d| render(d, src, Style { color: false })).collect()
}

/// Os diagnósticos do texto: o código, a gravidade, a mensagem, o trecho (em unidades
/// UTF-16) e o texto inteiro, como o terminal mostra.
#[wasm_bindgen]
pub fn check(source: &str) -> String {
    let a = lavra::analyze(source);
    let src = SourceFile::new(NAME, source.to_string());
    let list: Vec<Value> = a
        .diagnostics
        .iter()
        .map(|d| {
            json!({
                "code": d.code,
                "severity": if d.is_error() { "error" } else { "warning" },
                "message": d.message,
                "from": utf16(source, d.span.start),
                "to": utf16(source, d.span.end.max(d.span.start)),
                "rendered": render(d, &src, Style { color: false }),
            })
        })
        .collect();
    Value::Array(list).to_string()
}

/// A saída do programa, entregue ao JavaScript a cada escrita.
struct Output<'a>(&'a js_sys::Function);

impl std::io::Write for Output<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.0.call1(&JsValue::NULL, &JsValue::from_str(&String::from_utf8_lossy(buf)));
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Roda o programa com a entrada dada. A saída vai para `on_output` enquanto o programa
/// roda; no fim, volta como terminou ("ok", "errors" ou "failed"), os erros e avisos como o
/// terminal mostra (`stderr`), a falha de execução sozinha (`failure`, que a página mostra
/// junto da saída; os erros e avisos ela já mostra ao conferir) e o código de saída.
#[wasm_bindgen]
pub fn run(source: &str, input: &str, on_output: &js_sys::Function) -> String {
    let src = SourceFile::new(NAME, source.to_string());
    let result = match lavra::compile(source) {
        Err(diags) => json!({ "status": "errors", "stderr": shown(&diags, &src), "exit": 1 }),
        Ok((prog, warnings)) => {
            let mut machine = lavra::interp::Machine::new(&prog, Output(on_output), input.as_bytes());
            machine.limit_depth(DEPTH);
            match machine.run() {
                Ok(()) => json!({ "status": "ok", "stderr": shown(&warnings, &src), "exit": 0 }),
                Err(f) => {
                    let failure = render(&f.diagnostic(), &src, Style { color: false });
                    let stderr = shown(&warnings, &src) + &failure;
                    json!({ "status": "failed", "stderr": stderr, "failure": failure, "exit": f.exit_code() })
                }
            }
        }
    };
    result.to_string()
}
