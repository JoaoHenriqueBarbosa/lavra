# Morfologia: tabelas, verificação e decisões

Cinco arquivos autocontidos em `src/morphology/` foram preenchidos em 2026-10-22 por
agentes paralelos, cada um conferido contra uma referência do português inteiro, com os
casos de borda trazidos para decisão. As verificações ficam em `tools/morphology/`
(venv `uv` com `verbecc` e `spylls`, Hunspell pt_BR do VERO em `data/`; ver o README de
lá para preparar o ambiente).

| Arquivo | Referência | Resultado |
|---|---|---|
| `verbs/first_person.rs` | Verbiste, 15 267 verbos | 0 divergência; 12 regras, 3 formas soltas, 97 defectivos |
| `verbs/subjunctive.rs` | Verbiste, 11 716 verbos | 0 divergência; 18 regras de família |
| `verbs/third_person.rs` | Verbiste, 11 715 verbos | 0 divergência; 79 irregulares, 17 famílias, 23 hiatos |
| `nouns/plural.rs` | Hunspell, 286 391 palavras | 8 divergências (erros do dicionário); compostos 80,6% |
| `adjectives/feminine.rs` | Hunspell + particípios, 136 571 pares | 0 divergência fora das 297 documentadas |

Os erros da referência (modelos corrompidos do Verbiste, flags erradas do VERO) foram
corrigidos do lado da referência, cada um documentado no checker correspondente.

## Decisões tomadas

- **Defectivos sem 1ª pessoa** (abolir, falir, trovejar...): `None`; no cabeçalho é erro,
  com a saída de declarar a forma entre parênteses.
- **1ª pessoa igual a palavra da língua** ("como", "caso", "salvo", "pelo", ordinais,
  nomes de tipo e de operação): o verbo não pode ser definido.
- **Dois verbos com a mesma 1ª pessoa** (vender/vendar, fundar/fundir): erro só se os
  dois estiverem no mesmo programa.
- **"ser" e "ir" dão "for"**: predicado com "ir" é proibido.
- **Plural "-em" do subjuntivo e do infinitivo pessoal** leva o acento do hiato
  ("caírem", "construírem", mas "seguirem"); estava sem acento em 94 verbos.
- **Impessoais no plural** ("nevarem") são aceitos.
- **Inversão da 3ª pessoa** (dica "Escreve" → "Escrevo"): candidatos só entre verbos
  conhecidos, conferidos pela ida; a heurística antiga errava 1 242 formas.
- **Defectivos na 3ª pessoa**: `None` em falir, precaver, reaver, remir; o tipo abolir
  mantém a forma regular.
- **Singular a partir do plural**: não se faz; "Para cada nota" pluraliza o item e procura
  a coleção declarada. Nome invariável ("lápis") exige a coleção explícita.
- **Gênero**: regra mais tabela para "-ão" (-ã ou -ona); listas de "-eso" e "-eio";
  Cunha e Cintra para montês e pedrês (uniformes). A volta ao masculino é estrita
  ("o pino for acesa" é erro), por isso adjetivo uniforme em "-a" fora das listas
  (hermafrodita) sai errado.

## Pendente (decisão adiada)

- Pronominais: hoje ênclise ("queixo-me"); o natural no Brasil seria próclise.
- Formas abundantes (averiguo/averíguo, negocio/negoceio, constrói/construi): só a
  primeira é aceita.
- Plurais duplos (anciões/anciãos, corrimões/corrimãos): só um é gerado; reconhecer
  todos pediria `plurals() -> Vec<String>`.
- Compostos com hífen: heurística (80,6%); alternativa seria aceitar qualquer combinação
  ou pedir o plural declarado.
- Grafia anterior ao Acordo ("tranqüilo"): passa sem normalizar.
- Colisões de 3ª pessoa com palavras funcionais ("para", "nada", "como"): sem efeito
  hoje (os verbos de dado-sujeito são fechados); se o usuário puder definir verbos de
  dado-sujeito, os que colidem precisam ser proibidos.
- Nome cujo plural é palavra reservada ("vez" → "vezes").
- Compostos de cor ("azul-marinho") e de dois adjetivos ("surdo-mudo").
