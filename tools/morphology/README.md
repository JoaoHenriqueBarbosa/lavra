# Verificação da morfologia contra o dicionário

Cada arquivo de tabela em `src/morphology` é autocontido e é conferido aqui contra
uma referência do português inteiro:

| Arquivo | Referência |
|---|---|
| `verbs/first_person.rs` | Verbiste (via `verbecc`): 15 267 verbos, 331 modelos |
| `verbs/subjunctive.rs` | idem |
| `verbs/third_person.rs` | idem |
| `nouns/plural.rs` | Hunspell pt_BR (VERO, LibreOffice), via `spylls` |
| `adjectives/feminine.rs` | idem |

Preparar o ambiente (uma vez):

```bash
cd tools/morphology
uv venv .venv && uv pip install --python .venv/bin/python verbecc spylls
mkdir -p data
curl -sSfL -o data/pt_BR.dic https://raw.githubusercontent.com/LibreOffice/dictionaries/master/pt_BR/pt_BR.dic
curl -sSfL -o data/pt_BR.aff https://raw.githubusercontent.com/LibreOffice/dictionaries/master/pt_BR/pt_BR.aff
```

Cada `check_*.py` compila um binário pequeno que inclui o arquivo Rust por `#[path]`,
passa todas as palavras da referência por ele e lista as divergências.
