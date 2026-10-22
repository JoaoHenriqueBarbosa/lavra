/* Lavra — biblioteca de execução para programas compilados em x86-64.
   Entrada e saída no formato da linguagem (vírgula decimal, verdadeiro/falso),
   controle de reservas de memória e mensagens de erro de execução. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <ctype.h>

static int eco = 0;
typedef struct { void *p; int64_t n; } bloco_t;
static bloco_t *vivos = NULL; static int64_t nvivos = 0, capvivos = 0;
static void **mortos = NULL; static int64_t nmortos = 0, capmortos = 0;

void lavra_inicia(void) { eco = getenv("LAVRA_ECO") != NULL; }

__attribute__((noreturn)) void lavra_falha(int64_t linha, const char *msg) {
  fflush(stdout);
  fprintf(stderr, "\n[Erro de execução] linha %lld: %s\n", (long long)linha, msg);
  exit(1);
}
void lavra_falha_indice(int64_t linha, int64_t i, int64_t n) {
  char b[200];
  snprintf(b, sizeof b, "Posição %lld fora da sequência de %lld itens (válidas: 0 a %lld).", (long long)i, (long long)n, (long long)(n - 1));
  lavra_falha(linha, b);
}
void lavra_termina(void) {
  fflush(stdout);
  if (nvivos) {
    int64_t t = 0;
    for (int64_t k = 0; k < nvivos; k++) t += vivos[k].n;
    fprintf(stderr, "[aviso] %lld bloco(s) reservado(s) com \"Reservo\" não foram liberados (%lld bytes).\n", (long long)nvivos, (long long)t);
  }
}

/* ---------- saída */
static void formata_real(double v, char *out) {
  if (isnan(v)) { strcpy(out, "indefinido"); return; }
  if (isinf(v)) { strcpy(out, v > 0 ? "infinito" : "-infinito"); return; }
  char t[64], s[64];
  snprintf(t, sizeof t, "%.12g", v);
  double x = strtod(t, NULL);
  int p;
  for (p = 1; p <= 17; p++) { snprintf(s, sizeof s, "%.*g", p, x); if (strtod(s, NULL) == x) break; }
  double ax = fabs(x);
  char *e = strchr(s, 'e');
  if (e && ax >= 1e-6 && ax < 1e21) {
    if (ax >= 1) snprintf(s, sizeof s, "%.0f", x);
    else { int ex = (int)floor(log10(ax)); snprintf(s, sizeof s, "%.*f", p - 1 - ex, x); }
  } else if (e) {
    char sinal = e[1]; int ex = atoi(e + 2);
    sprintf(e, "e%c%d", sinal, ex);
  }
  if (strcmp(s, "-0") == 0) strcpy(s, "0");
  for (char *c = s; *c; c++) if (*c == '.') *c = ',';
  strcpy(out, s);
}
static int utf8(uint32_t c, char *b) {
  if (c < 0x80) { b[0] = c; return 1; }
  if (c < 0x800) { b[0] = 0xC0 | (c >> 6); b[1] = 0x80 | (c & 63); return 2; }
  if (c < 0x10000) { b[0] = 0xE0 | (c >> 12); b[1] = 0x80 | ((c >> 6) & 63); b[2] = 0x80 | (c & 63); return 3; }
  b[0] = 0xF0 | (c >> 18); b[1] = 0x80 | ((c >> 12) & 63); b[2] = 0x80 | ((c >> 6) & 63); b[3] = 0x80 | (c & 63); return 4;
}
void lavra_escreve_i(int64_t v) { printf("%lld", (long long)v); }
void lavra_escreve_u(uint64_t v) { printf("%llu", (unsigned long long)v); }
void lavra_escreve_r(double v) { char b[64]; formata_real(v, b); fputs(b, stdout); }
void lavra_escreve_b(int64_t v) { fputs(v ? "verdadeiro" : "falso", stdout); }
void lavra_escreve_c(uint32_t c) { char b[5]; int n = utf8(c, b); fwrite(b, 1, n, stdout); }
void lavra_escreve_t(const char *s) { fputs(s ? s : "", stdout); }
void lavra_escreve_p(void *p) { if (!p) fputs("nada", stdout); else printf("0x%06llx", (unsigned long long)(uintptr_t)p); }
void lavra_nl(void) { putchar('\n'); }

/* descritor de tipo: i64 u8 r64 b c t p  [N*tam:elem]  {nome@desloc=tipo,...} */
static const char *escreve_desc(FILE *f, const char *d, const unsigned char *p) {
  char *fim;
  switch (*d) {
    case 'i': case 'u': {
      int bits = (int)strtol(d + 1, &fim, 10);
      if (*d == 'i') {
        int64_t v = bits == 8 ? *(int8_t *)p : bits == 16 ? *(int16_t *)p : bits == 32 ? *(int32_t *)p : *(int64_t *)p;
        fprintf(f, "%lld", (long long)v);
      } else {
        uint64_t v = bits == 8 ? *(uint8_t *)p : bits == 16 ? *(uint16_t *)p : bits == 32 ? *(uint32_t *)p : *(uint64_t *)p;
        fprintf(f, "%llu", (unsigned long long)v);
      }
      return fim;
    }
    case 'r': {
      int bits = (int)strtol(d + 1, &fim, 10);
      char b[64]; formata_real(bits == 32 ? *(float *)p : *(double *)p, b); fputs(b, f);
      return fim;
    }
    case 'b': fputs(*p ? "verdadeiro" : "falso", f); return d + 1;
    case 'c': { char b[5]; int n = utf8(*(uint32_t *)p, b); fwrite(b, 1, n, f); return d + 1; }
    case 't': fputs(*(char **)p ? *(char **)p : "", f); return d + 1;
    case 'p': { void *q = *(void **)p; if (!q) fputs("nada", f); else fprintf(f, "0x%06llx", (unsigned long long)(uintptr_t)q); return d + 1; }
    case '[': {
      long n = strtol(d + 1, &fim, 10);
      long tam = strtol(fim + 1, &fim, 10);
      const char *elem = fim + 1, *depois = elem;
      fputc('[', f);
      for (long k = 0; k < n; k++) { if (k) fputs("; ", f); depois = escreve_desc(f, elem, p + k * tam); }
      fputc(']', f);
      return depois + 1;
    }
    case '{': {
      const char *c = d + 1;
      fputc('{', f);
      for (int primeiro = 1;; primeiro = 0) {
        const char *arroba = strchr(c, '@');
        if (!primeiro) fputs("; ", f);
        fwrite(c, 1, arroba - c, f); fputs(": ", f);
        long off = strtol(arroba + 1, &fim, 10);
        c = escreve_desc(f, fim + 1, p + off);
        if (*c == ',') { c++; continue; }
        break;
      }
      fputc('}', f);
      return c + 1;
    }
  }
  return d + 1;
}
void lavra_escreve_bloco(const void *p, const char *desc) { escreve_desc(stdout, desc, p); }

/* ---------- textos */
static char *copia(const char *s) { char *r = malloc(strlen(s) + 1); strcpy(r, s); return r; }
char *lavra_txt_i(int64_t v) { char b[32]; snprintf(b, sizeof b, "%lld", (long long)v); return copia(b); }
char *lavra_txt_u(uint64_t v) { char b[32]; snprintf(b, sizeof b, "%llu", (unsigned long long)v); return copia(b); }
char *lavra_txt_r(double v) { char b[64]; formata_real(v, b); return copia(b); }
char *lavra_txt_b(int64_t v) { return copia(v ? "verdadeiro" : "falso"); }
char *lavra_txt_c(uint32_t c) { char b[5] = {0}; utf8(c, b); return copia(b); }
char *lavra_txt_p(void *p) { char b[32]; if (!p) strcpy(b, "nada"); else snprintf(b, sizeof b, "0x%06llx", (unsigned long long)(uintptr_t)p); return copia(b); }
char *lavra_txt_bloco(const void *p, const char *desc) {
  char *buf = NULL; size_t n = 0;
  FILE *f = open_memstream(&buf, &n);
  escreve_desc(f, desc, p); fclose(f);
  return buf;
}
char *lavra_concatena(const char *a, const char *b) {
  size_t x = strlen(a), y = strlen(b);
  char *r = malloc(x + y + 1);
  memcpy(r, a, x); memcpy(r + x, b, y + 1);
  return r;
}
int64_t lavra_tamanho_texto(const char *s) { int64_t n = 0; for (; *s; s++) if ((*s & 0xC0) != 0x80) n++; return n; }

/* ---------- entrada e conversões */
static char *apara(char *s) {
  while (isspace((unsigned char)*s)) s++;
  char *e = s + strlen(s);
  while (e > s && isspace((unsigned char)e[-1])) *--e = 0;
  return s;
}
static char *le_linha(int64_t linha) {
  static char *buf = NULL; static size_t cap = 0;
  fflush(stdout);
  ssize_t n = getline(&buf, &cap, stdin);
  if (n < 0) lavra_falha(linha, "A entrada acabou: o programa pediu mais um valor e não havia.");
  while (n > 0 && (buf[n - 1] == '\n' || buf[n - 1] == '\r')) buf[--n] = 0;
  if (eco) { fputs(buf, stdout); putchar('\n'); }
  return buf;
}
static int64_t para_i(const char *orig, int64_t linha, const char *onde) {
  char tmp[256], *o = tmp; const char *s;
  char copia_[256]; strncpy(copia_, orig, 255); copia_[255] = 0;
  s = apara(copia_);
  for (const char *c = s; *c && o - tmp < 250; c++) if (*c != ' ' && *c != '_') *o++ = *c;
  *o = 0;
  const char *c = tmp; if (*c == '-') c++;
  int ok = *c != 0;
  for (; *c; c++) if (!isdigit((unsigned char)*c)) ok = 0;
  if (!ok) { char m[400]; snprintf(m, sizeof m, "%s: esperava um número inteiro, veio \"%s\".", onde, s); lavra_falha(linha, m); }
  return (int64_t)strtoull(tmp[0] == '-' ? tmp + 1 : tmp, NULL, 10) * (tmp[0] == '-' ? -1 : 1);
}
static double para_r(const char *orig, int64_t linha, const char *onde) {
  char tmp[256], *o = tmp, copia_[256];
  strncpy(copia_, orig, 255); copia_[255] = 0;
  char *s = apara(copia_);
  for (const char *c = s; *c && o - tmp < 250; c++) if (*c != ' ') *o++ = (*c == ',') ? '.' : *c;
  *o = 0;
  const char *c = tmp; if (*c == '-') c++;
  int dig = 0, ponto = 0, ok = 1, depois = 0;
  for (; *c; c++) {
    if (isdigit((unsigned char)*c)) { dig++; if (ponto) depois++; }
    else if (*c == '.' && !ponto) ponto = 1;
    else ok = 0;
  }
  if (!dig || (ponto && !depois)) ok = 0;
  if (!ok) { char m[400]; snprintf(m, sizeof m, "%s: esperava um número real (como 7,5), veio \"%s\".", onde, s); lavra_falha(linha, m); }
  return strtod(tmp, NULL);
}
static uint32_t para_c(const char *orig, int64_t linha, const char *onde) {
  char copia_[256]; strncpy(copia_, orig, 255); copia_[255] = 0;
  const unsigned char *s = (const unsigned char *)apara(copia_);
  if (!*s) { char m[200]; snprintf(m, sizeof m, "%s: esperava um caractere e veio linha vazia.", onde); lavra_falha(linha, m); }
  if (s[0] < 0x80) return s[0];
  if ((s[0] & 0xE0) == 0xC0) return ((s[0] & 31) << 6) | (s[1] & 63);
  if ((s[0] & 0xF0) == 0xE0) return ((s[0] & 15) << 12) | ((s[1] & 63) << 6) | (s[2] & 63);
  return ((s[0] & 7) << 18) | ((s[1] & 63) << 12) | ((s[2] & 63) << 6) | (s[3] & 63);
}
static int64_t para_b(const char *orig, int64_t linha, const char *onde) {
  char copia_[256]; strncpy(copia_, orig, 255); copia_[255] = 0;
  char *s = apara(copia_);
  for (char *c = s; *c; c++) *c = tolower((unsigned char)*c);
  if (!strcmp(s, "verdadeiro") || !strcmp(s, "v") || !strcmp(s, "sim") || !strcmp(s, "s")) return 1;
  if (!strcmp(s, "falso") || !strcmp(s, "f") || !strcmp(s, "não") || !strcmp(s, "nao") || !strcmp(s, "n")) return 0;
  char m[400]; snprintf(m, sizeof m, "%s: esperava verdadeiro/falso (ou sim/não), veio \"%s\".", onde, s);
  lavra_falha(linha, m);
}
int64_t lavra_le_i(int64_t linha) { return para_i(le_linha(linha), linha, "Leitura"); }
double lavra_le_r(int64_t linha) { return para_r(le_linha(linha), linha, "Leitura"); }
uint32_t lavra_le_c(int64_t linha) { return para_c(le_linha(linha), linha, "Leitura"); }
int64_t lavra_le_b(int64_t linha) { return para_b(le_linha(linha), linha, "Leitura"); }
char *lavra_le_t(int64_t linha) { return copia(le_linha(linha)); }
int64_t lavra_txt_para_i(const char *s, int64_t linha) { return para_i(s, linha, "Conversão"); }
double lavra_txt_para_r(const char *s, int64_t linha) { return para_r(s, linha, "Conversão"); }
uint32_t lavra_txt_para_c(const char *s, int64_t linha) { return para_c(s, linha, "Conversão"); }
int64_t lavra_txt_para_b(const char *s, int64_t linha) { return para_b(s, linha, "Conversão"); }

/* ---------- aritmética */
int64_t lavra_pot_int(int64_t b, int64_t e, int64_t linha) {
  if (e < 0) lavra_falha(linha, "Potência inteira com expoente negativo; use reais.");
  uint64_t r = 1, x = (uint64_t)b, k = (uint64_t)e;
  while (k) { if (k & 1) r *= x; x *= x; k >>= 1; }
  return (int64_t)r;
}

/* ---------- memória */
void *lavra_reserva(int64_t n, int64_t tam, int64_t linha) {
  if (n <= 0) { char m[200]; snprintf(m, sizeof m, "Reserva de %lld itens: a quantidade precisa ser positiva.", (long long)n); lavra_falha(linha, m); }
  void *p = calloc((size_t)n, (size_t)tam);
  if (!p) lavra_falha(linha, "Memória esgotada: reservas demais.");
  if (nvivos == capvivos) { capvivos = capvivos ? capvivos * 2 : 64; vivos = realloc(vivos, capvivos * sizeof *vivos); }
  vivos[nvivos++] = (bloco_t){ p, n * tam };
  for (int64_t k = 0; k < nmortos; k++) if (mortos[k] == p) { mortos[k] = mortos[--nmortos]; break; }
  return p;
}
void lavra_libera(void *p, int64_t linha) {
  if (!p) lavra_falha(linha, "\"Libero\" recebeu nada.");
  for (int64_t k = 0; k < nvivos; k++) if (vivos[k].p == p) {
    vivos[k] = vivos[--nvivos];
    if (nmortos == capmortos) { capmortos = capmortos ? capmortos * 2 : 64; mortos = realloc(mortos, capmortos * sizeof *mortos); }
    mortos[nmortos++] = p;
    free(p);
    return;
  }
  char m[200];
  for (int64_t k = 0; k < nmortos; k++) if (mortos[k] == p) {
    snprintf(m, sizeof m, "O endereço %p já tinha sido liberado (liberação dupla).", p);
    lavra_falha(linha, m);
  }
  snprintf(m, sizeof m, "Só se libera o início de um bloco que veio de \"Reservo\"; %p não é.", p);
  lavra_falha(linha, m);
}
