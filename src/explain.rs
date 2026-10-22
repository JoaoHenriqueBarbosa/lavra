//! Os textos de `lavrac explain LXXXX`: um arquivo `src/explain/LXXXX.md` por código, com
//! a explicação, um exemplo errado e o certo. O `build.rs` embute todos; um teste confere
//! que o exemplo errado dá o código e o certo não dá nada.

include!(concat!(env!("OUT_DIR"), "/explain.rs"));

/// O texto da explicação de um código ("L0334" ou "334").
pub fn text(id: &str) -> Option<&'static str> {
    let code = crate::codes::find(id)?;
    TEXTS.iter().find(|(c, _)| *c == code.id).map(|(_, t)| *t)
}

/// Um exemplo de uma explicação: o programa e a entrada que ele lê, se houver.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Example {
    pub program: String,
    pub input: String,
}

/// Os exemplos errado e certo de um texto (vazios quando o código não tem exemplo).
pub fn examples(text: &str) -> (Option<Example>, Option<Example>) {
    let section = |title: &str| -> Option<Example> {
        let start = text.find(&format!("## {title}"))?;
        let rest = &text[start..];
        let end = rest[3..].find("\n## ").map_or(rest.len(), |i| i + 3);
        let body = &rest[..end];
        let program = fenced(body, "lavra")?;
        let input = fenced(body, "entrada").unwrap_or_default();
        Some(Example { program, input })
    };
    (section("Errado"), section("Certo"))
}

/// O conteúdo do primeiro bloco cercado com a etiqueta dada.
fn fenced(body: &str, tag: &str) -> Option<String> {
    let open = format!("```{tag}\n");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find("```")? + start;
    Some(body[start..end].to_string())
}
