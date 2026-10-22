//! Gerador de código nativo: HIR para Cranelift, objeto para linkar com o runtime.
//!
//! Escalares vivem em variáveis SSA; locais confiados a outra função vão para um slot de
//! pilha (o endereço é o que se passa). Dentro de listas, registros e "talvez", e em todo
//! lugar endereçável (slot, global, parâmetro confiado), o valor ocupa uma casa de 8 bytes.
//! Cada local com dono tem uma bandeira de posse: liga quando recebe valor, desliga quando
//! entrega; na saída da função, o que ainda tem dono é liberado.

use crate::ast::{BinOp, ParamMode, Rel, UnOp};
use crate::builtins::Builtin;
use crate::codes::{Code, L0900, L0904, L0905};
use crate::hir::{self, Cond, ExprKind, PlaceProj, Stmt, StmtKind, Var};
use crate::span::Span;
use crate::textfmt;
use crate::types::{IntTy, Ty};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{self, AbiParam, BlockArg, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, isa::OwnedTargetIsa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

const PTR: types::Type = types::I64;

/// Funções do runtime: nome, parâmetros, retornos.
const RUNTIME: &[(&str, &[types::Type], &[types::Type])] = &[
    ("rt_fault", &[types::I32, PTR, types::I64, types::I32, types::I32], &[]),
    ("rt_fault_abs", &[types::I64, types::I32, types::I32, types::I32, types::I32], &[]),
    ("rt_fault_shift", &[types::I64, types::I32, types::I32, types::I32, types::I32], &[]),
    ("rt_fault_sqrt", &[types::F64, types::I32, types::I32], &[]),
    ("rt_fault_convert_int", &[types::I64, types::I32, types::I32, types::I32, types::I32, types::I32], &[]),
    ("rt_fault_convert_real", &[types::F64, types::I32, types::I32, types::I32, types::I32], &[]),
    ("rt_failure_new", &[PTR, types::I32, types::I32], &[PTR]),
    ("rt_failure_drop", &[PTR], &[]),
    ("rt_enter", &[types::I32, types::I32], &[]),
    ("rt_exit", &[], &[]),
    ("rt_write", &[PTR, types::I64], &[]),
    ("rt_newline", &[], &[]),
    ("rt_text_lit", &[PTR, types::I64], &[PTR]),
    ("rt_text_new", &[], &[PTR]),
    ("rt_text_append", &[PTR, PTR, types::I64], &[]),
    ("rt_text_char", &[PTR, types::I64, types::I32, types::I32], &[types::I32]),
    ("rt_text_last_char", &[PTR, types::I32, types::I32], &[types::I32]),
    ("rt_text_chars", &[PTR], &[PTR]),
    ("rt_list_new", &[], &[PTR]),
    ("rt_list_push", &[PTR, types::I64], &[]),
    ("rt_list_len", &[PTR], &[types::I64]),
    ("rt_list_data", &[PTR], &[PTR]),
    ("rt_list_addr", &[PTR, types::I64, types::I32, types::I32], &[PTR]),
    ("rt_list_last_addr", &[PTR, types::I32, types::I32], &[PTR]),
    ("rt_list_take", &[PTR, types::I64, types::I32, types::I32], &[types::I64]),
    ("rt_record_new", &[types::I64], &[PTR]),
    ("rt_box", &[types::I64], &[PTR]),
    ("rt_drop", &[PTR, types::I64], &[]),
    ("rt_clone", &[PTR, types::I64], &[types::I64]),
    ("rt_cmp", &[PTR, types::I64, types::I64], &[types::I32]),
    ("rt_len", &[PTR, types::I64], &[types::I64]),
    ("rt_pow_int", &[types::I64, types::I64, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
    ("rt_pow_real", &[types::F64, types::F64], &[types::F64]),
    ("rt_fmod", &[types::F64, types::F64], &[types::F64]),
    ("rt_read_int", &[types::I32, types::I32, PTR, types::I64, types::I32, types::I32], &[types::I64]),
    ("rt_read_real", &[PTR, types::I64, types::I32, types::I32], &[types::F64]),
    ("rt_read_bool", &[PTR, types::I64, types::I32, types::I32], &[types::I8]),
    ("rt_read_char", &[PTR, types::I64, types::I32, types::I32], &[types::I32]),
    ("rt_read_text", &[types::I32, types::I32], &[PTR]),
];

/// Compila o programa para um objeto (ELF) com `lavra_init`, `lavra_main` e o fonte
/// embutido para as mensagens de erro.
pub fn compile(prog: &hir::Program, src: &str, path: &str) -> Result<Vec<u8>, String> {
    let isa = isa()?;
    let builder = ObjectBuilder::new(isa, "lavra", cranelift_module::default_libcall_names()).map_err(|e| e.to_string())?;
    let mut cg = Codegen {
        module: ObjectModule::new(builder),
        runtime: HashMap::new(),
        builtins: HashMap::new(),
        funcs: Vec::new(),
        globals: Vec::new(),
        descs: HashMap::new(),
        strings: HashMap::new(),
    };
    for (name, params, rets) in RUNTIME {
        let mut sig = cg.module.make_signature();
        sig.params.extend(params.iter().map(|t| AbiParam::new(*t)));
        sig.returns.extend(rets.iter().map(|t| AbiParam::new(*t)));
        let id = cg.module.declare_function(name, Linkage::Import, &sig).map_err(|e| e.to_string())?;
        cg.runtime.insert(name, id);
    }
    for (i, f) in prog.funcs.iter().enumerate() {
        let (name, linkage) =
            if Some(i) == prog.main { ("lavra_main".to_string(), Linkage::Export) } else { (format!("lavra_f{i}"), Linkage::Local) };
        let sig = cg.signature(f);
        let id = cg.module.declare_function(&name, linkage, &sig).map_err(|e| e.to_string())?;
        cg.funcs.push(id);
    }
    for (i, _) in prog.globals.iter().enumerate() {
        let id = cg.module.declare_data(&format!("lavra_g{i}"), Linkage::Local, true, false).map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        d.define_zeroinit(8);
        d.set_align(8);
        cg.module.define_data(id, &d).map_err(|e| e.to_string())?;
        cg.globals.push(id);
    }
    cg.export_bytes("lavra_src", src.as_bytes())?;
    cg.export_bytes("lavra_path", path.as_bytes())?;
    cg.export_u64("lavra_src_len", src.len() as u64)?;
    cg.export_u64("lavra_path_len", path.len() as u64)?;

    let mut fctx = FunctionBuilderContext::new();
    let init_id = {
        let mut sig = cg.module.make_signature();
        sig.returns.clear();
        cg.module.declare_function("lavra_init", Linkage::Export, &sig).map_err(|e| e.to_string())?
    };
    cg.define(prog, &prog.init, init_id, &mut fctx)?;
    for (i, f) in prog.funcs.iter().enumerate() {
        cg.define(prog, f, cg.funcs[i], &mut fctx)?;
    }
    let product = cg.module.finish();
    product.emit().map_err(|e| e.to_string())
}

fn isa() -> Result<OwnedTargetIsa, String> {
    let mut flags = settings::builder();
    flags.set("opt_level", "speed").map_err(|e| e.to_string())?;
    flags.set("is_pic", "true").map_err(|e| e.to_string())?;
    let builder = cranelift_native::builder().map_err(|e| e.to_string())?;
    builder.finish(settings::Flags::new(flags)).map_err(|e| e.to_string())
}

struct Codegen {
    module: ObjectModule,
    runtime: HashMap<&'static str, FuncId>,
    /// As operações da própria língua já declaradas.
    builtins: HashMap<Builtin, FuncId>,
    funcs: Vec<FuncId>,
    globals: Vec<DataId>,
    descs: HashMap<String, DataId>,
    strings: HashMap<String, DataId>,
}

/// Tipo nativo de um valor da Lavra (`None` para "nada").
fn clif(t: &Ty) -> Option<types::Type> {
    Some(match t {
        Ty::Int(i) => int_type(*i),
        Ty::Real(32) => types::F32,
        Ty::Real(_) => types::F64,
        Ty::Bool => types::I8,
        Ty::Char => types::I32,
        Ty::Text | Ty::List(_) | Ty::Record(_) | Ty::Maybe(_) | Ty::Enum(_) => PTR,
        Ty::Unit => return None,
        Ty::Infer(_) | Ty::Error => panic!("tipo não resolvido no gerador de código: {t:?}"),
    })
}

fn int_type(i: IntTy) -> types::Type {
    match i.bits {
        8 => types::I8,
        16 => types::I16,
        32 => types::I32,
        _ => types::I64,
    }
}

fn type_key(t: &Ty) -> String {
    match t {
        Ty::Int(i) => format!("i{}{}", i.bits, if i.signed { "s" } else { "u" }),
        Ty::Real(b) => format!("r{b}"),
        Ty::Bool => "b".into(),
        Ty::Char => "c".into(),
        Ty::Text => "t".into(),
        Ty::List(e) => format!("l({})", type_key(e)),
        Ty::Record(r) => format!("rec{}", r.id),
        Ty::Enum(e) => format!("enum{}", e.id),
        Ty::Maybe(e) => format!("m({})", type_key(e)),
        other => panic!("descritor de {other:?}"),
    }
}

impl Codegen {
    fn signature(&self, f: &hir::Func) -> ir::Signature {
        let mut sig = self.module.make_signature();
        for (id, mode) in &f.params {
            let t = if *mode == ParamMode::Lent { PTR } else { clif(&f.locals[*id].ty).unwrap() };
            sig.params.push(AbiParam::new(t));
        }
        if let Some(t) = clif(&f.ret) {
            sig.returns.push(AbiParam::new(t));
        }
        if f.fallible {
            sig.returns.push(AbiParam::new(PTR));
        }
        sig
    }

    fn export_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let id = self.module.declare_data(name, Linkage::Export, false, false).map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        let mut v = bytes.to_vec();
        v.push(0);
        d.define(v.into_boxed_slice());
        self.module.define_data(id, &d).map_err(|e| e.to_string())
    }

    fn export_u64(&mut self, name: &str, v: u64) -> Result<(), String> {
        let id = self.module.declare_data(name, Linkage::Export, false, false).map_err(|e| e.to_string())?;
        let mut d = DataDescription::new();
        d.define(v.to_le_bytes().to_vec().into_boxed_slice());
        d.set_align(8);
        self.module.define_data(id, &d).map_err(|e| e.to_string())
    }

    fn string(&mut self, s: &str) -> DataId {
        if let Some(id) = self.strings.get(s) {
            return *id;
        }
        let id = self.module.declare_anonymous_data(false, false).unwrap();
        let mut d = DataDescription::new();
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        d.define(v.into_boxed_slice());
        self.module.define_data(id, &d).unwrap();
        self.strings.insert(s.to_string(), id);
        id
    }

    /// Descritor estático do tipo (mesmo layout de `Desc` no runtime).
    fn desc(&mut self, t: &Ty) -> DataId {
        let key = type_key(t);
        if let Some(id) = self.descs.get(&key) {
            return *id;
        }
        let id = self.module.declare_anonymous_data(false, false).unwrap();
        self.descs.insert(key, id);
        let (kind, bits, signed, feminine) = match t {
            Ty::Int(i) => (0u32, i.bits as u32, i.signed as u32, 0u32),
            Ty::Real(b) => (1, *b as u32, 0, 0),
            Ty::Bool => (2, 0, 0, 0),
            Ty::Char => (3, 0, 0, 0),
            Ty::Text => (4, 0, 0, 0),
            Ty::List(_) => (5, 0, 0, 1),
            Ty::Record(r) => (6, 0, 0, r.feminine as u32),
            Ty::Maybe(_) => (7, 0, 0, 0),
            Ty::Enum(e) => (8, 0, 0, e.feminine as u32),
            _ => unreachable!(),
        };
        let mut bytes = vec![0u8; 56];
        bytes[0..4].copy_from_slice(&kind.to_le_bytes());
        bytes[4..8].copy_from_slice(&bits.to_le_bytes());
        bytes[8..12].copy_from_slice(&signed.to_le_bytes());
        bytes[12..16].copy_from_slice(&feminine.to_le_bytes());
        let mut d = DataDescription::new();
        d.set_align(8);
        let mut relocs: Vec<(u32, DataId)> = Vec::new();
        match t {
            Ty::List(e) | Ty::Maybe(e) => relocs.push((16, self.desc(e))),
            Ty::Enum(e) => {
                bytes[24..32].copy_from_slice(&(e.variants.len() as u64).to_le_bytes());
                bytes[48..56].copy_from_slice(&(e.name.len() as u64).to_le_bytes());
                relocs.push((40, self.string(&e.name)));
                // Um descritor por caso: nome, gênero, campos.
                let vid = self.module.declare_anonymous_data(false, false).unwrap();
                let mut vb = vec![0u8; 40 * e.variants.len().max(1)];
                let mut vd = DataDescription::new();
                vd.set_align(8);
                let mut vrel = Vec::new();
                for (i, v) in e.variants.iter().enumerate() {
                    let o = i * 40;
                    vb[o + 8..o + 16].copy_from_slice(&(v.name.len() as u64).to_le_bytes());
                    vb[o + 16..o + 24].copy_from_slice(&(v.feminine as u64).to_le_bytes());
                    vb[o + 24..o + 32].copy_from_slice(&(v.fields.len() as u64).to_le_bytes());
                    vrel.push((o as u32, self.string(&v.name)));
                    let fid = self.fields_desc(&v.fields, &v.field_articles);
                    vrel.push((o as u32 + 32, fid));
                }
                vd.define(vb.into_boxed_slice());
                for (off, target) in vrel {
                    let gv = self.module.declare_data_in_data(target, &mut vd);
                    vd.write_data_addr(off, gv, 0);
                }
                self.module.define_data(vid, &vd).unwrap();
                relocs.push((32, vid));
            }
            Ty::Record(r) => {
                bytes[24..32].copy_from_slice(&(r.fields.len() as u64).to_le_bytes());
                bytes[48..56].copy_from_slice(&(r.name.len() as u64).to_le_bytes());
                relocs.push((40, self.string(&r.name)));
                let fid = self.fields_desc(&r.fields, &r.field_articles);
                relocs.push((32, fid));
            }
            _ => {}
        }
        d.define(bytes.into_boxed_slice());
        for (off, target) in relocs {
            let gv = self.module.declare_data_in_data(target, &mut d);
            d.write_data_addr(off, gv, 0);
        }
        self.module.define_data(id, &d).unwrap();
        id
    }

    /// Lista de descritores de campo (mesmo layout de `FieldDesc` no runtime).
    fn fields_desc(&mut self, fields: &[(String, Ty)], articles: &[(String, &'static str)]) -> DataId {
        let fid = self.module.declare_anonymous_data(false, false).unwrap();
        let mut fb = vec![0u8; 40 * fields.len().max(1)];
        let mut fd = DataDescription::new();
        fd.set_align(8);
        let mut frel = Vec::new();
        for (i, (name, fty)) in fields.iter().enumerate() {
            let art = articles[i].1;
            let o = i * 40;
            fb[o + 16..o + 24].copy_from_slice(&(name.len() as u64).to_le_bytes());
            fb[o + 32..o + 40].copy_from_slice(&(art.len() as u64).to_le_bytes());
            frel.push((o as u32, self.desc(fty)));
            frel.push((o as u32 + 8, self.string(name)));
            frel.push((o as u32 + 24, self.string(art)));
        }
        fd.define(fb.into_boxed_slice());
        for (off, target) in frel {
            let gv = self.module.declare_data_in_data(target, &mut fd);
            fd.write_data_addr(off, gv, 0);
        }
        self.module.define_data(fid, &fd).unwrap();
        fid
    }

    fn define(&mut self, prog: &hir::Program, f: &hir::Func, id: FuncId, fctx: &mut FunctionBuilderContext) -> Result<(), String> {
        let mut ctx = Context::new();
        ctx.func.signature = if std::ptr::eq(f, &prog.init) { self.module.make_signature() } else { self.signature(f) };
        let frontend = self.module.isa().frontend_config();
        {
            let b = FunctionBuilder::new(&mut ctx.func, fctx);
            let mut fx = Fx::new(self, prog, f, b);
            fx.body();
            fx.b.seal_all_blocks();
            fx.b.finalize(frontend);
        }
        self.module.define_function(id, &mut ctx).map_err(|e| format!("{e:?}\n{}", ctx.func.display()))
    }
}

#[derive(Clone, Copy)]
enum Storage {
    Var(Variable),
    /// Slot de pilha com a casa de 8 bytes (local cujo endereço é confiado).
    Slot(ir::StackSlot),
    /// Parâmetro confiado: a variável guarda o endereço da casa do chamador.
    Ref(Variable),
}

struct Fx<'a, 'b> {
    cg: &'a mut Codegen,
    prog: &'a hir::Program,
    f: &'a hir::Func,
    b: FunctionBuilder<'b>,
    storage: Vec<Storage>,
    /// Bandeira de posse de cada local com dono.
    owned: Vec<Option<Variable>>,
    loops: Vec<(ir::Block, ir::Block)>,
    /// Valores temporários com dono, a liberar no fim da oração.
    temps: Vec<(Value, Ty)>,
    refs: HashMap<FuncId, ir::FuncRef>,
}

fn flags() -> MemFlagsData {
    MemFlagsData::trusted()
}

fn span_args(s: Span) -> (i64, i64) {
    (s.start as i64, s.end as i64)
}

impl<'a, 'b> Fx<'a, 'b> {
    fn new(cg: &'a mut Codegen, prog: &'a hir::Program, f: &'a hir::Func, mut b: FunctionBuilder<'b>) -> Self {
        // Locais confiados a outra função precisam de endereço.
        let mut addressed = vec![false; f.locals.len()];
        mark_addressed(&f.body, &mut addressed);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let mut storage = Vec::new();
        let mut owned = Vec::new();
        for (i, l) in f.locals.iter().enumerate() {
            let param = f.params.iter().find(|(id, _)| *id == i).map(|(_, m)| *m);
            let st = if param == Some(ParamMode::Lent) {
                Storage::Ref(b.declare_var(PTR))
            } else if addressed[i] {
                Storage::Slot(b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3)))
            } else {
                match clif(&l.ty) {
                    Some(t) => Storage::Var(b.declare_var(t)),
                    None => Storage::Var(b.declare_var(types::I8)),
                }
            };
            storage.push(st);
            let has_owner = !l.ty.is_copy() && !l.borrowed && !matches!(param, Some(ParamMode::Lent | ParamMode::Borrowed));
            owned.push(if has_owner {
                let v = b.declare_var(types::I8);
                let z = b.ins().iconst(types::I8, 0);
                b.def_var(v, z);
                Some(v)
            } else {
                None
            });
        }
        // Variáveis escalares sem valor inicial recebem zero (o Cranelift exige definição).
        for (i, l) in f.locals.iter().enumerate() {
            if let Storage::Var(v) = storage[i]
                && !f.params.iter().any(|(id, _)| *id == i)
            {
                let t = clif(&l.ty).unwrap_or(types::I8);
                let z = zero(&mut b, t);
                b.def_var(v, z);
            }
        }
        let params: Vec<Value> = b.block_params(entry).to_vec();
        let mut fx = Fx { cg, prog, f, b, storage, owned, loops: Vec::new(), temps: Vec::new(), refs: HashMap::new() };
        for ((id, _), v) in f.params.iter().zip(params) {
            match fx.storage[*id] {
                Storage::Var(var) | Storage::Ref(var) => fx.b.def_var(var, v),
                Storage::Slot(ss) => {
                    let s = fx.slot_of(v, &f.locals[*id].ty);
                    fx.b.ins().stack_store(PTR, s, ss, 0);
                }
            }
            if let Some(flag) = fx.owned[*id] {
                let one = fx.b.ins().iconst(types::I8, 1);
                fx.b.def_var(flag, one);
            }
        }
        fx
    }

    fn body(&mut self) {
        self.block(&self.f.body);
        if !self.b.is_unreachable() {
            if self.f.ret != Ty::Unit {
                self.b.ins().trap(ir::TrapCode::unwrap_user(1));
            } else {
                self.drop_locals();
                self.ret(None, None);
            }
        }
    }

    // --------------------------------------------------------- utilidades

    fn rt(&mut self, name: &'static str, args: &[Value]) -> Vec<Value> {
        let id = self.cg.runtime[name];
        let r = *self.refs.entry(id).or_insert_with(|| self.cg.module.declare_func_in_func(id, self.b.func));
        let call = self.b.ins().call(r, args);
        self.b.inst_results(call).to_vec()
    }

    fn rt1(&mut self, name: &'static str, args: &[Value]) -> Value {
        self.rt(name, args)[0]
    }

    fn data_addr(&mut self, id: DataId) -> Value {
        let gv = self.cg.module.declare_data_in_func(id, self.b.func);
        self.b.ins().symbol_value(PTR, gv)
    }

    fn str_args(&mut self, s: &str) -> (Value, Value) {
        let id = self.cg.string(s);
        let p = self.data_addr(id);
        let n = self.b.ins().iconst(types::I64, s.len() as i64);
        (p, n)
    }

    fn desc(&mut self, t: &Ty) -> Value {
        let id = self.cg.desc(t);
        self.data_addr(id)
    }

    fn span_vals(&mut self, s: Span) -> (Value, Value) {
        let (lo, hi) = span_args(s);
        (self.b.ins().iconst(types::I32, lo), self.b.ins().iconst(types::I32, hi))
    }

    fn fault(&mut self, code: Code, msg: &str, span: Span) {
        let c = self.b.ins().iconst(types::I32, code.number() as i64);
        let (p, n) = self.str_args(msg);
        let (lo, hi) = self.span_vals(span);
        self.rt("rt_fault", &[c, p, n, lo, hi]);
        self.b.ins().trap(ir::TrapCode::unwrap_user(2));
    }

    /// Se `cond` for verdadeiro, falha com a mensagem; senão segue.
    fn fault_if(&mut self, cond: Value, code: Code, msg: &str, span: Span) {
        let bad = self.b.create_block();
        let ok = self.b.create_block();
        self.b.ins().brif(cond, bad, &[], ok, &[]);
        self.b.switch_to_block(bad);
        self.fault(code, msg, span);
        self.b.switch_to_block(ok);
    }

    fn slot_of(&mut self, v: Value, t: &Ty) -> Value {
        match t {
            Ty::Int(i) if i.bits < 64 => {
                if i.signed {
                    self.b.ins().sextend(types::I64, v)
                } else {
                    self.b.ins().uextend(types::I64, v)
                }
            }
            Ty::Real(32) => {
                let i = self.b.ins().bitcast(types::I32, MemFlagsData::new(), v);
                self.b.ins().uextend(types::I64, i)
            }
            Ty::Real(_) => self.b.ins().bitcast(types::I64, MemFlagsData::new(), v),
            Ty::Bool | Ty::Char => self.b.ins().uextend(types::I64, v),
            _ => v,
        }
    }

    fn value_of(&mut self, s: Value, t: &Ty) -> Value {
        match t {
            Ty::Int(i) if i.bits < 64 => self.b.ins().ireduce(int_type(*i), s),
            Ty::Real(32) => {
                let i = self.b.ins().ireduce(types::I32, s);
                self.b.ins().bitcast(types::F32, MemFlagsData::new(), i)
            }
            Ty::Real(_) => self.b.ins().bitcast(types::F64, MemFlagsData::new(), s),
            Ty::Bool => self.b.ins().ireduce(types::I8, s),
            Ty::Char => self.b.ins().ireduce(types::I32, s),
            _ => s,
        }
    }

    fn drop_value(&mut self, v: Value, t: &Ty) {
        if t.is_copy() {
            return;
        }
        let d = self.desc(t);
        let s = self.slot_of(v, t);
        self.rt("rt_drop", &[d, s]);
    }

    fn flush_temps(&mut self, mark: usize) {
        while self.temps.len() > mark {
            let (v, t) = self.temps.pop().unwrap();
            self.drop_value(v, &t);
        }
    }

    // --------------------------------------------------------------- locais

    fn read_local(&mut self, id: usize) -> Value {
        let t = self.f.locals[id].ty.clone();
        match self.storage[id] {
            Storage::Var(v) => self.b.use_var(v),
            Storage::Slot(ss) => {
                let s = self.b.ins().stack_load(PTR, types::I64, ss, 0);
                self.value_of(s, &t)
            }
            Storage::Ref(v) => {
                let p = self.b.use_var(v);
                let s = self.b.ins().load(types::I64, flags(), p, 0);
                self.value_of(s, &t)
            }
        }
    }

    fn write_local(&mut self, id: usize, val: Value) {
        let t = self.f.locals[id].ty.clone();
        match self.storage[id] {
            Storage::Var(v) => self.b.def_var(v, val),
            Storage::Slot(ss) => {
                let s = self.slot_of(val, &t);
                self.b.ins().stack_store(PTR, s, ss, 0);
            }
            Storage::Ref(v) => {
                let p = self.b.use_var(v);
                let s = self.slot_of(val, &t);
                self.b.ins().store(flags(), s, p, 0);
            }
        }
    }

    /// Dá valor novo ao local, liberando o que ele tinha (se tinha dono).
    fn assign_local(&mut self, id: usize, val: Value) {
        let t = self.f.locals[id].ty.clone();
        if let Some(flag) = self.owned[id] {
            let has = self.b.use_var(flag);
            let drop_b = self.b.create_block();
            let cont = self.b.create_block();
            self.b.ins().brif(has, drop_b, &[], cont, &[]);
            self.b.switch_to_block(drop_b);
            let old = self.read_local(id);
            self.drop_value(old, &t);
            self.b.ins().jump(cont, &[]);
            self.b.switch_to_block(cont);
            self.write_local(id, val);
            let one = self.b.ins().iconst(types::I8, 1);
            self.b.def_var(flag, one);
        } else if let Storage::Ref(_) = self.storage[id]
            && !t.is_copy()
        {
            let old = self.read_local(id);
            self.drop_value(old, &t);
            self.write_local(id, val);
        } else {
            self.write_local(id, val);
        }
    }

    fn global_addr(&mut self, g: usize) -> Value {
        let id = self.cg.globals[g];
        self.data_addr(id)
    }

    fn read_var(&mut self, v: Var) -> Value {
        match v {
            Var::Local(id) => self.read_local(id),
            Var::Global(g) => {
                let t = self.prog.globals[g].ty.clone();
                let p = self.global_addr(g);
                let s = self.b.ins().load(types::I64, flags(), p, 0);
                self.value_of(s, &t)
            }
        }
    }

    fn drop_locals(&mut self) {
        for id in 0..self.f.locals.len() {
            if let Some(flag) = self.owned[id] {
                let t = self.f.locals[id].ty.clone();
                let has = self.b.use_var(flag);
                let drop_b = self.b.create_block();
                let cont = self.b.create_block();
                self.b.ins().brif(has, drop_b, &[], cont, &[]);
                self.b.switch_to_block(drop_b);
                let old = self.read_local(id);
                self.drop_value(old, &t);
                self.b.ins().jump(cont, &[]);
                self.b.switch_to_block(cont);
            }
        }
    }

    fn ret(&mut self, value: Option<Value>, failure: Option<Value>) {
        let mut out = Vec::new();
        if let Some(t) = clif(&self.f.ret) {
            out.push(value.unwrap_or_else(|| zero(&mut self.b, t)));
        }
        if self.f.fallible {
            out.push(failure.unwrap_or_else(|| self.b.ins().iconst(PTR, 0)));
        }
        self.b.ins().return_(&out);
        let dead = self.b.create_block();
        self.b.switch_to_block(dead);
    }

    /// Sai da função com a falha, liberando tudo o que é dela.
    fn fail_out(&mut self, failure: Value) {
        let temps = std::mem::take(&mut self.temps);
        for (v, t) in temps.iter().rev() {
            self.drop_value(*v, t);
        }
        self.temps = temps;
        self.drop_locals();
        self.ret(None, Some(failure));
    }

    // -------------------------------------------------------------- lugares

    /// Tipo de cada passo do caminho do lugar, da raiz para fora.
    fn step_types(&self, p: &hir::Place) -> Vec<Ty> {
        let mut t = match p.root {
            Var::Local(id) => self.f.locals[id].ty.clone(),
            Var::Global(g) => self.prog.globals[g].ty.clone(),
        };
        let mut out = vec![t.clone()];
        for pr in &p.proj {
            t = match (pr, &t) {
                (PlaceProj::Field(i), Ty::Record(r)) => r.fields[*i].1.clone(),
                (_, Ty::List(e)) => (**e).clone(),
                (_, Ty::Text) => Ty::Char,
                _ => unreachable!(),
            };
            out.push(t.clone());
        }
        out
    }

    /// Valor do contêiner que guarda o último passo, e o tipo dele.
    fn parent(&mut self, p: &hir::Place) -> (Value, Ty) {
        let tys = self.step_types(p);
        let mut cur = self.read_var(p.root);
        for (k, pr) in p.proj[..p.proj.len() - 1].iter().enumerate() {
            let addr = self.step_addr(cur, &tys[k], pr);
            let s = self.b.ins().load(types::I64, flags(), addr, 0);
            cur = self.value_of(s, &tys[k + 1]);
        }
        (cur, tys[p.proj.len() - 1].clone())
    }

    fn step_addr(&mut self, container: Value, container_ty: &Ty, pr: &PlaceProj) -> Value {
        match pr {
            PlaceProj::Field(i) => self.b.ins().iadd_imm_s(container, (*i as i64) * 8),
            PlaceProj::Index(e, span) => {
                let n = self.expr(e, false).unwrap();
                let (lo, hi) = self.span_vals(*span);
                debug_assert!(matches!(container_ty, Ty::List(_)));
                self.rt1("rt_list_addr", &[container, n, lo, hi])
            }
            PlaceProj::Last(span) => {
                let (lo, hi) = self.span_vals(*span);
                self.rt1("rt_list_last_addr", &[container, lo, hi])
            }
        }
    }

    /// Lê o lugar (emprestado: quem lê não fica dono).
    fn read_place(&mut self, p: &hir::Place) -> Value {
        if p.proj.is_empty() {
            return self.read_var(p.root);
        }
        let (container, cty) = self.parent(p);
        let last = p.proj.last().unwrap();
        if cty == Ty::Text {
            return match last {
                PlaceProj::Index(e, span) => {
                    let n = self.expr(e, false).unwrap();
                    let (lo, hi) = self.span_vals(*span);
                    self.rt1("rt_text_char", &[container, n, lo, hi])
                }
                PlaceProj::Last(span) => {
                    let (lo, hi) = self.span_vals(*span);
                    self.rt1("rt_text_last_char", &[container, lo, hi])
                }
                PlaceProj::Field(_) => unreachable!(),
            };
        }
        let addr = self.step_addr(container, &cty, last);
        let s = self.b.ins().load(types::I64, flags(), addr, 0);
        self.value_of(s, &p.ty)
    }

    /// Endereço da casa do lugar (para confiar, mudar ou substituir).
    fn place_addr(&mut self, p: &hir::Place) -> Value {
        if p.proj.is_empty() {
            let Var::Local(id) = p.root else { unreachable!("constante não se confia") };
            return match self.storage[id] {
                Storage::Slot(ss) => self.b.ins().stack_addr(PTR, ss, 0),
                Storage::Ref(v) => self.b.use_var(v),
                Storage::Var(_) => unreachable!("local confiado sem endereço"),
            };
        }
        let (container, cty) = self.parent(p);
        self.step_addr(container, &cty, p.proj.last().unwrap())
    }

    fn write_place(&mut self, p: &hir::Place, val: Value) {
        if p.proj.is_empty() {
            match p.root {
                Var::Local(id) => self.assign_local(id, val),
                Var::Global(g) => {
                    let t = self.prog.globals[g].ty.clone();
                    let addr = self.global_addr(g);
                    let s = self.slot_of(val, &t);
                    self.b.ins().store(flags(), s, addr, 0);
                }
            }
            return;
        }
        let addr = self.place_addr(p);
        if !p.ty.is_copy() {
            let old = self.b.ins().load(types::I64, flags(), addr, 0);
            let d = self.desc(&p.ty);
            self.rt("rt_drop", &[d, old]);
        }
        let s = self.slot_of(val, &p.ty);
        self.b.ins().store(flags(), s, addr, 0);
    }

    // ------------------------------------------------------------- orações

    fn block(&mut self, b: &hir::Block) {
        for s in &b.0 {
            if self.b.is_unreachable() {
                break;
            }
            let mark = self.temps.len();
            self.stmt(s);
            self.flush_temps(mark);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Error => unreachable!("programa com erro não chega aos back-ends"),
            StmtKind::Let { var, value } => {
                let v = self.expr(value, true).unwrap();
                match var {
                    Var::Local(id) => self.assign_local(*id, v),
                    Var::Global(g) => {
                        let t = self.prog.globals[*g].ty.clone();
                        let addr = self.global_addr(*g);
                        let s = self.slot_of(v, &t);
                        self.b.ins().store(flags(), s, addr, 0);
                    }
                }
            }
            StmtKind::Assign { place, value } => {
                let v = self.expr(value, true).unwrap();
                self.write_place(place, v);
            }
            StmtKind::Compound { place, op, value, span } => {
                let rhs = self.expr(value, false).unwrap();
                let cur = self.read_place(place);
                let r = self.arith(*op, cur, rhs, &place.ty, &value.ty, *span);
                self.write_place(place, r);
            }
            StmtKind::Push { place, value } => {
                let list = self.read_place(place);
                let v = self.expr(value, true).unwrap();
                let Ty::List(elem) = &place.ty else { unreachable!() };
                let s = self.slot_of(v, elem);
                self.rt("rt_list_push", &[list, s]);
            }
            StmtKind::Write { value, newline } => {
                let v = self.expr(value, false).unwrap();
                let d = self.desc(&value.ty);
                let s = self.slot_of(v, &value.ty);
                self.rt("rt_write", &[d, s]);
                if *newline {
                    self.rt("rt_newline", &[]);
                }
            }
            StmtKind::If { arms, otherwise } => {
                let end = self.b.create_block();
                for (c, body) in arms {
                    let yes = self.b.create_block();
                    let no = self.b.create_block();
                    let v = self.cond(c);
                    self.b.ins().brif(v, yes, &[], no, &[]);
                    self.b.switch_to_block(yes);
                    self.block(body);
                    if !self.b.is_unreachable() {
                        self.b.ins().jump(end, &[]);
                    }
                    self.b.switch_to_block(no);
                }
                if let Some(o) = otherwise {
                    self.block(o);
                }
                if !self.b.is_unreachable() {
                    self.b.ins().jump(end, &[]);
                }
                self.b.switch_to_block(end);
            }
            StmtKind::While { cond, body } => {
                let head = self.b.create_block();
                let inner = self.b.create_block();
                let exit = self.b.create_block();
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let v = self.cond(cond);
                self.b.ins().brif(v, inner, &[], exit, &[]);
                self.b.switch_to_block(inner);
                self.loop_body(body, head, exit);
                self.b.switch_to_block(exit);
            }
            StmtKind::Repeat { count, body } => {
                let head = self.b.create_block();
                let inner = self.b.create_block();
                let next = self.b.create_block();
                let exit = self.b.create_block();
                match count {
                    None => {
                        self.b.ins().jump(head, &[]);
                        self.b.switch_to_block(head);
                        self.b.ins().jump(inner, &[]);
                        self.b.switch_to_block(inner);
                        self.loop_body(body, next, exit);
                        self.b.switch_to_block(next);
                        self.b.ins().jump(head, &[]);
                    }
                    Some(c) => {
                        let n = self.expr(c, false).unwrap();
                        let Ty::Int(it) = c.ty else { unreachable!() };
                        let t = int_type(it);
                        let i = self.b.declare_var(t);
                        let z = self.b.ins().iconst(t, 0);
                        self.b.def_var(i, z);
                        self.b.ins().jump(head, &[]);
                        self.b.switch_to_block(head);
                        let iv = self.b.use_var(i);
                        let cc = if it.signed { IntCC::SignedLessThan } else { IntCC::UnsignedLessThan };
                        let go = self.b.ins().icmp(cc, iv, n);
                        self.b.ins().brif(go, inner, &[], exit, &[]);
                        self.b.switch_to_block(inner);
                        self.loop_body(body, next, exit);
                        self.b.switch_to_block(next);
                        let iv = self.b.use_var(i);
                        let i1 = self.b.ins().iadd_imm_s(iv, 1);
                        self.b.def_var(i, i1);
                        self.b.ins().jump(head, &[]);
                    }
                }
                self.b.switch_to_block(exit);
            }
            StmtKind::ForRange { var, from, to, body } => {
                let a = self.expr(from, false).unwrap();
                let z = self.expr(to, false).unwrap();
                let Ty::Int(it) = from.ty else { unreachable!() };
                let t = int_type(it);
                let le = if it.signed { IntCC::SignedLessThanOrEqual } else { IntCC::UnsignedLessThanOrEqual };
                let up = self.b.ins().icmp(le, a, z);
                let one = self.b.ins().iconst(t, 1);
                let minus = self.b.ins().iconst(t, -1);
                let step = self.b.ins().select(up, one, minus);
                let i = self.b.declare_var(t);
                self.b.def_var(i, a);
                let head = self.b.create_block();
                let next = self.b.create_block();
                let exit = self.b.create_block();
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let iv = self.b.use_var(i);
                self.write_local(*var, iv);
                self.loop_body(body, next, exit);
                self.b.switch_to_block(next);
                let iv = self.b.use_var(i);
                let done = self.b.ins().icmp(IntCC::Equal, iv, z);
                let more = self.b.create_block();
                self.b.ins().brif(done, exit, &[], more, &[]);
                self.b.switch_to_block(more);
                let iv = self.b.use_var(i);
                let i1 = self.b.ins().iadd(iv, step);
                self.b.def_var(i, i1);
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(exit);
            }
            StmtKind::ForEach { var, source, body } => {
                let src = self.expr(source, false).unwrap();
                let list = if source.ty == Ty::Text {
                    let l = self.rt1("rt_text_chars", &[src]);
                    self.temps.push((l, Ty::List(Box::new(Ty::Char))));
                    l
                } else {
                    src
                };
                let elem = match &source.ty {
                    Ty::List(e) => (**e).clone(),
                    _ => Ty::Char,
                };
                let len = self.rt1("rt_list_len", &[list]);
                let data = self.rt1("rt_list_data", &[list]);
                let i = self.b.declare_var(types::I64);
                let z = self.b.ins().iconst(types::I64, 0);
                self.b.def_var(i, z);
                let head = self.b.create_block();
                let inner = self.b.create_block();
                let next = self.b.create_block();
                let exit = self.b.create_block();
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let iv = self.b.use_var(i);
                let go = self.b.ins().icmp(IntCC::UnsignedLessThan, iv, len);
                self.b.ins().brif(go, inner, &[], exit, &[]);
                self.b.switch_to_block(inner);
                let off = self.b.ins().imul_imm_s(iv, 8);
                let addr = self.b.ins().iadd(data, off);
                let s = self.b.ins().load(types::I64, flags(), addr, 0);
                let item = self.value_of(s, &elem);
                self.write_local(*var, item);
                self.loop_body(body, next, exit);
                self.b.switch_to_block(next);
                let iv = self.b.use_var(i);
                let i1 = self.b.ins().iadd_imm_s(iv, 1);
                self.b.def_var(i, i1);
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(exit);
            }
            StmtKind::Break | StmtKind::Continue => {
                let (cont, brk) = *self.loops.last().unwrap();
                let target = if matches!(s.kind, StmtKind::Break) { brk } else { cont };
                self.b.ins().jump(target, &[]);
                let dead = self.b.create_block();
                self.b.switch_to_block(dead);
            }
            StmtKind::Return(v) => {
                let val = v.as_ref().and_then(|e| self.expr(e, true));
                let temps = std::mem::take(&mut self.temps);
                for (v, t) in temps.iter().rev() {
                    self.drop_value(*v, t);
                }
                self.temps = temps;
                self.drop_locals();
                self.ret(val, None);
            }
            StmtKind::Match { scrutinee, arms, otherwise } => {
                let v = self.expr(scrutinee, false).unwrap();
                let tag = self.b.ins().load(types::I64, flags(), v, 0);
                let Ty::Enum(e) = &scrutinee.ty else { unreachable!() };
                let end = self.b.create_block();
                for arm in arms {
                    let yes = self.b.create_block();
                    let no = self.b.create_block();
                    let mut hit = self.b.ins().icmp_imm_s(IntCC::Equal, tag, arm.tags[0] as i64);
                    for t in &arm.tags[1..] {
                        let h = self.b.ins().icmp_imm_s(IntCC::Equal, tag, *t as i64);
                        hit = self.b.ins().bor(hit, h);
                    }
                    self.b.ins().brif(hit, yes, &[], no, &[]);
                    self.b.switch_to_block(yes);
                    for (id, i) in &arm.binds {
                        let fty = e.variants[arm.tags[0]].fields[*i].1.clone();
                        let s = self.b.ins().load(types::I64, flags(), v, ((i + 1) * 8) as i32);
                        let fv = self.value_of(s, &fty);
                        self.write_local(*id, fv);
                    }
                    self.block(&arm.body);
                    if !self.b.is_unreachable() {
                        self.b.ins().jump(end, &[]);
                    }
                    self.b.switch_to_block(no);
                }
                if let Some(o) = otherwise {
                    self.block(o);
                }
                if !self.b.is_unreachable() {
                    self.b.ins().jump(end, &[]);
                }
                self.b.switch_to_block(end);
            }
            StmtKind::Desist(m) => {
                let text = self.expr(m, true).unwrap();
                let (lo, hi) = self.span_vals(m.span);
                let f = self.rt1("rt_failure_new", &[text, lo, hi]);
                self.fail_out(f);
            }
            StmtKind::Expr(e) => {
                if let Some(v) = self.expr(e, true) {
                    self.drop_value(v, &e.ty);
                }
            }
        }
    }

    fn loop_body(&mut self, body: &hir::Block, cont: ir::Block, exit: ir::Block) {
        self.loops.push((cont, exit));
        self.block(body);
        if !self.b.is_unreachable() {
            self.b.ins().jump(cont, &[]);
        }
        self.loops.pop();
    }

    // ------------------------------------------------------------ condições

    fn cond(&mut self, c: &Cond) -> Value {
        match c {
            Cond::And(v) | Cond::Or(v) => {
                let is_and = matches!(c, Cond::And(_));
                let end = self.b.create_block();
                self.b.append_block_param(end, types::I8);
                for (k, c) in v.iter().enumerate() {
                    let r = self.cond(c);
                    if k + 1 == v.len() {
                        self.b.ins().jump(end, &[BlockArg::Value(r)]);
                    } else {
                        let next = self.b.create_block();
                        if is_and {
                            self.b.ins().brif(r, next, &[], end, &[BlockArg::Value(r)]);
                        } else {
                            self.b.ins().brif(r, end, &[BlockArg::Value(r)], next, &[]);
                        }
                        self.b.switch_to_block(next);
                    }
                }
                self.b.switch_to_block(end);
                self.b.block_params(end)[0]
            }
            Cond::Not(c) => {
                let r = self.cond(c);
                self.b.ins().bxor_imm_s(r, 1)
            }
            Cond::Cmp { rel, lhs, rhs } => {
                let mark = self.temps.len();
                let a = self.expr(lhs, false).unwrap();
                let b = self.expr(rhs, false).unwrap();
                let r = self.compare(*rel, a, b, &lhs.ty);
                self.flush_temps(mark);
                r
            }
            Cond::Test(e) => {
                let mark = self.temps.len();
                let v = self.expr(e, true).unwrap();
                self.flush_temps(mark);
                self.b.ins().icmp_imm_s(IntCC::NotEqual, v, 0)
            }
            Cond::IsSome(e) => {
                let v = self.expr(e, false).unwrap();
                self.b.ins().icmp_imm_s(IntCC::NotEqual, v, 0)
            }
            Cond::IsVariant(e, tag) => {
                let mark = self.temps.len();
                let v = self.expr(e, false).unwrap();
                let t = self.b.ins().load(types::I64, flags(), v, 0);
                let r = self.b.ins().icmp_imm_s(IntCC::Equal, t, *tag as i64);
                self.flush_temps(mark);
                r
            }
        }
    }

    fn compare(&mut self, rel: Rel, a: Value, b: Value, t: &Ty) -> Value {
        match t {
            Ty::Int(it) => {
                let cc = match (rel, it.signed) {
                    (Rel::Greater, true) => IntCC::SignedGreaterThan,
                    (Rel::Greater, false) => IntCC::UnsignedGreaterThan,
                    (Rel::Less, true) => IntCC::SignedLessThan,
                    (Rel::Less, false) => IntCC::UnsignedLessThan,
                    (Rel::AtLeast, true) => IntCC::SignedGreaterThanOrEqual,
                    (Rel::AtLeast, false) => IntCC::UnsignedGreaterThanOrEqual,
                    (Rel::AtMost, true) => IntCC::SignedLessThanOrEqual,
                    (Rel::AtMost, false) => IntCC::UnsignedLessThanOrEqual,
                    (Rel::Equal, _) => IntCC::Equal,
                    (Rel::Different, _) => IntCC::NotEqual,
                };
                self.b.ins().icmp(cc, a, b)
            }
            Ty::Bool | Ty::Char => {
                let cc = match rel {
                    Rel::Greater => IntCC::UnsignedGreaterThan,
                    Rel::Less => IntCC::UnsignedLessThan,
                    Rel::AtLeast => IntCC::UnsignedGreaterThanOrEqual,
                    Rel::AtMost => IntCC::UnsignedLessThanOrEqual,
                    Rel::Equal => IntCC::Equal,
                    Rel::Different => IntCC::NotEqual,
                };
                self.b.ins().icmp(cc, a, b)
            }
            Ty::Real(_) => {
                let cc = match rel {
                    Rel::Greater => FloatCC::GreaterThan,
                    Rel::Less => FloatCC::LessThan,
                    Rel::AtLeast => FloatCC::GreaterThanOrEqual,
                    Rel::AtMost => FloatCC::LessThanOrEqual,
                    Rel::Equal => FloatCC::Equal,
                    Rel::Different => FloatCC::NotEqual,
                };
                self.b.ins().fcmp(cc, a, b)
            }
            _ => {
                let d = self.desc(t);
                let code = self.rt1("rt_cmp", &[d, a, b]);
                let is = |fx: &mut Self, k: i64| fx.b.ins().icmp_imm_s(IntCC::Equal, code, k);
                match rel {
                    Rel::Equal => is(self, 0),
                    Rel::Different => self.b.ins().icmp_imm_s(IntCC::NotEqual, code, 0),
                    Rel::Greater => is(self, 1),
                    Rel::Less => is(self, -1),
                    Rel::AtLeast => {
                        let x = is(self, 0);
                        let y = is(self, 1);
                        self.b.ins().bor(x, y)
                    }
                    Rel::AtMost => {
                        let x = is(self, 0);
                        let y = is(self, -1);
                        self.b.ins().bor(x, y)
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------- expressões

    /// Avalia a expressão. Com `owned`, o valor sai com dono para quem pediu (um local
    /// lido é entregue); sem, quem pediu só lê, e valores novos com dono viram
    /// temporários liberados no fim da oração.
    fn expr(&mut self, e: &hir::Expr, owned: bool) -> Option<Value> {
        let fresh = |fx: &mut Self, v: Value| -> Option<Value> {
            if !owned && !e.ty.is_copy() {
                fx.temps.push((v, e.ty.clone()));
            }
            Some(v)
        };
        match &e.kind {
            ExprKind::Int(v) => {
                let t = clif(&e.ty).unwrap();
                Some(match t {
                    types::F64 => self.b.ins().f64const(*v as f64),
                    types::F32 => self.b.ins().f32const(*v as f32),
                    _ => self.b.ins().iconst(t, *v as i64),
                })
            }
            ExprKind::Real(v) => Some(if e.ty == Ty::Real(32) { self.b.ins().f32const(*v as f32) } else { self.b.ins().f64const(*v) }),
            ExprKind::Bool(v) => Some(self.b.ins().iconst(types::I8, *v as i64)),
            ExprKind::Char(c) => Some(self.b.ins().iconst(types::I32, *c as i64)),
            ExprKind::Text(s) => {
                let (p, n) = self.str_args(s);
                let v = self.rt1("rt_text_lit", &[p, n]);
                fresh(self, v)
            }
            ExprKind::Read(p) => {
                let v = self.read_place(p);
                if owned
                    && !e.ty.is_copy()
                    && let Var::Local(id) = p.root
                    && p.proj.is_empty()
                {
                    // Entrega: o local deixa de ser dono.
                    if let Some(flag) = self.owned[id] {
                        let z = self.b.ins().iconst(types::I8, 0);
                        self.b.def_var(flag, z);
                    } else {
                        // Vista emprestada não se entrega (o verificador garante); copia por segurança.
                        let d = self.desc(&e.ty);
                        return Some(self.rt1("rt_clone", &[d, v]));
                    }
                }
                Some(v)
            }
            ExprKind::Binary(op, l, r) => {
                let a = self.expr(l, false).unwrap();
                let b = self.expr(r, false).unwrap();
                Some(self.arith(*op, a, b, &e.ty, &r.ty, e.span))
            }
            ExprKind::Unary(op, a) => {
                let v = self.expr(a, false).unwrap();
                Some(self.unary(*op, v, &a.ty, e.span))
            }
            ExprKind::Concat(parts) => {
                let t = self.rt1("rt_text_new", &[]);
                for p in parts {
                    let mark = self.temps.len();
                    let v = self.expr(p, false).unwrap();
                    let d = self.desc(&p.ty);
                    let s = self.slot_of(v, &p.ty);
                    self.rt("rt_text_append", &[t, d, s]);
                    self.flush_temps(mark);
                }
                fresh(self, t)
            }
            ExprKind::Input => {
                let (lo, hi) = self.span_vals(e.span);
                let v = match &e.ty {
                    Ty::Int(it) => {
                        let bits = self.b.ins().iconst(types::I32, it.bits as i64);
                        let signed = self.b.ins().iconst(types::I32, it.signed as i64);
                        let (p, n) = self.str_args(&e.ty.describe());
                        let r = self.rt1("rt_read_int", &[bits, signed, p, n, lo, hi]);
                        if it.bits < 64 { self.b.ins().ireduce(int_type(*it), r) } else { r }
                    }
                    Ty::Real(b) => {
                        let (p, n) = self.str_args(&e.ty.describe());
                        let r = self.rt1("rt_read_real", &[p, n, lo, hi]);
                        if *b == 32 { self.b.ins().fdemote(types::F32, r) } else { r }
                    }
                    Ty::Bool => {
                        let (p, n) = self.str_args(&e.ty.describe());
                        self.rt1("rt_read_bool", &[p, n, lo, hi])
                    }
                    Ty::Char => {
                        let (p, n) = self.str_args(&e.ty.describe());
                        self.rt1("rt_read_char", &[p, n, lo, hi])
                    }
                    Ty::Text => {
                        let v = self.rt1("rt_read_text", &[lo, hi]);
                        return fresh(self, v);
                    }
                    _ => unreachable!(),
                };
                Some(v)
            }
            ExprKind::Call { callee, args } => {
                let (v, _) = self.call(*callee, args, &e.ty, e.span);
                v.and_then(|v| fresh(self, v))
            }
            ExprKind::Try { call, fallback } => {
                let ExprKind::Call { callee, args } = &call.kind else { unreachable!() };
                let (v, fail) = self.call(*callee, args, &call.ty, call.span);
                let fail = fail.unwrap();
                let ok = self.b.create_block();
                let bad = self.b.create_block();
                let end = self.b.create_block();
                let rt = clif(&e.ty);
                if let Some(t) = rt {
                    self.b.append_block_param(end, t);
                }
                let is_fail = self.b.ins().icmp_imm_s(IntCC::NotEqual, fail, 0);
                self.b.ins().brif(is_fail, bad, &[], ok, &[]);
                self.b.switch_to_block(ok);
                let args_ok: Vec<BlockArg> = v.into_iter().map(BlockArg::Value).collect();
                self.b.ins().jump(end, &args_ok);
                self.b.switch_to_block(bad);
                match fallback {
                    None => self.fail_out(fail),
                    Some(fb) => {
                        self.rt("rt_failure_drop", &[fail]);
                        let mark = self.temps.len();
                        let mut fv = self.expr(fb, true).unwrap();
                        // O valor de reserva lido de um lugar vira cópia, para o resultado ter dono nos dois caminhos.
                        if !fb.ty.is_copy() && matches!(fb.kind, ExprKind::Read(_)) {
                            let d = self.desc(&fb.ty);
                            let s = self.slot_of(fv, &fb.ty);
                            let c = self.rt1("rt_clone", &[d, s]);
                            fv = self.value_of(c, &fb.ty);
                        }
                        self.flush_temps(mark);
                        self.b.ins().jump(end, &[BlockArg::Value(fv)]);
                    }
                }
                self.b.switch_to_block(end);
                let r = rt.map(|_| self.b.block_params(end)[0]);
                r.and_then(|v| fresh(self, v))
            }
            ExprKind::Fact(c) => Some(self.cond(c)),
            ExprKind::List(items) => {
                let l = self.rt1("rt_list_new", &[]);
                let Ty::List(elem) = &e.ty else { unreachable!() };
                for it in items {
                    let v = self.expr(it, true).unwrap();
                    let s = self.slot_of(v, elem);
                    self.rt("rt_list_push", &[l, s]);
                }
                fresh(self, l)
            }
            ExprKind::Record(fields) => {
                let n = self.b.ins().iconst(types::I64, fields.len() as i64);
                let r = self.rt1("rt_record_new", &[n]);
                for (i, f) in fields.iter().enumerate() {
                    let v = self.expr(f, true).unwrap();
                    let s = self.slot_of(v, &f.ty);
                    self.b.ins().store(flags(), s, r, (i * 8) as i32);
                }
                fresh(self, r)
            }
            ExprKind::Copy(a) => {
                let mark = self.temps.len();
                let v = self.expr(a, false).unwrap();
                let out = if a.ty.is_copy() {
                    v
                } else {
                    let d = self.desc(&a.ty);
                    let s = self.slot_of(v, &a.ty);
                    let c = self.rt1("rt_clone", &[d, s]);
                    self.value_of(c, &a.ty)
                };
                self.flush_temps(mark);
                fresh(self, out)
            }
            ExprKind::Take { place, index } => {
                let list = self.read_place(place);
                let n = match index {
                    Some(i) => self.expr(i, false).unwrap(),
                    None => self.b.ins().iconst(types::I64, 0),
                };
                let (lo, hi) = self.span_vals(e.span);
                let s = self.rt1("rt_list_take", &[list, n, lo, hi]);
                let v = self.value_of(s, &e.ty);
                fresh(self, v)
            }
            ExprKind::Nothing => {
                let v = self.b.ins().iconst(PTR, 0);
                Some(v)
            }
            ExprKind::Convert(a) => {
                let v = self.expr(a, false).unwrap();
                let out = self.convert(v, &a.ty, &e.ty, e.span);
                if e.ty == Ty::Text { fresh(self, out) } else { Some(out) }
            }
            ExprKind::Variant { tag, fields } => {
                let n = self.b.ins().iconst(types::I64, fields.len() as i64 + 1);
                let r = self.rt1("rt_record_new", &[n]);
                let t = self.b.ins().iconst(types::I64, *tag as i64);
                self.b.ins().store(flags(), t, r, 0);
                for (i, f) in fields.iter().enumerate() {
                    let v = self.expr(f, true).unwrap();
                    let s = self.slot_of(v, &f.ty);
                    self.b.ins().store(flags(), s, r, ((i + 1) * 8) as i32);
                }
                fresh(self, r)
            }
            ExprKind::VariantField { value, field } => {
                let p = self.expr(value, false).unwrap();
                let s = self.b.ins().load(types::I64, flags(), p, ((field + 1) * 8) as i32);
                Some(self.value_of(s, &e.ty))
            }
            ExprKind::Wrap(a) => {
                let v = self.expr(a, true).unwrap();
                let s = self.slot_of(v, &a.ty);
                let b = self.rt1("rt_box", &[s]);
                fresh(self, b)
            }
            ExprKind::Unwrap(a) => {
                let p = self.expr(a, false).unwrap();
                let s = self.b.ins().load(types::I64, flags(), p, 0);
                Some(self.value_of(s, &e.ty))
            }
        }
    }

    /// Chama a função; devolve o valor (se houver) e a falha (se a função pode falhar).
    fn call(&mut self, callee: hir::Callee, args: &[hir::Arg], ret: &Ty, span: Span) -> (Option<Value>, Option<Value>) {
        let mut vals = Vec::new();
        for a in args {
            let v = match a.mode {
                ParamMode::Lent => {
                    let p = a.place.as_ref().unwrap();
                    self.place_addr(p)
                }
                ParamMode::Owned => self.expr(&a.value, true).unwrap(),
                ParamMode::Borrowed => self.expr(&a.value, false).unwrap(),
            };
            vals.push(v);
        }
        let func = match callee {
            hir::Callee::User(f) => f,
            hir::Callee::Builtin(b) => return (self.builtin(b, args, &vals, ret, span), None),
        };
        let (lo, hi) = self.span_vals(span);
        self.rt("rt_enter", &[lo, hi]);
        let id = self.cg.funcs[func];
        let r = *self.refs.entry(id).or_insert_with(|| self.cg.module.declare_func_in_func(id, self.b.func));
        let call = self.b.ins().call(r, &vals);
        let res = self.b.inst_results(call).to_vec();
        self.rt("rt_exit", &[]);
        let callee = &self.prog.funcs[func];
        let mut it = res.into_iter();
        let v = if callee.ret != Ty::Unit { it.next() } else { None };
        let f = if callee.fallible { it.next() } else { None };
        (v, f)
    }

    /// Uma operação da própria língua: a função do runtime (`Builtin::symbol`), com a
    /// assinatura tirada dos tipos dos argumentos e do valor, e o trecho da chamada no fim
    /// para relatar uma falha. Uma operação nova não mexe aqui.
    fn builtin(&mut self, b: Builtin, args: &[hir::Arg], vals: &[Value], ret: &Ty, span: Span) -> Option<Value> {
        let id = match self.cg.builtins.get(&b) {
            Some(id) => *id,
            None => {
                let mut sig = self.cg.module.make_signature();
                for a in args {
                    let t = if a.mode == ParamMode::Lent { PTR } else { clif(&a.value.ty).expect("argumento com valor") };
                    sig.params.push(AbiParam::new(t));
                }
                sig.params.extend([AbiParam::new(types::I32), AbiParam::new(types::I32)]);
                sig.returns.extend(clif(ret).map(AbiParam::new));
                let id = self.cg.module.declare_function(b.symbol(), Linkage::Import, &sig).expect("a operação do runtime");
                self.cg.builtins.insert(b, id);
                id
            }
        };
        let r = *self.refs.entry(id).or_insert_with(|| self.cg.module.declare_func_in_func(id, self.b.func));
        let (lo, hi) = self.span_vals(span);
        let mut all = vals.to_vec();
        all.extend([lo, hi]);
        let call = self.b.ins().call(r, &all);
        self.b.inst_results(call).first().copied()
    }

    /// "x como T": mesma regra do interpretador (não cabe é falha, nunca truncamento).
    fn convert(&mut self, v: Value, from: &Ty, to: &Ty, span: Span) -> Value {
        match (from, to) {
            (_, Ty::Text) => {
                let t = self.rt1("rt_text_new", &[]);
                let d = self.desc(from);
                let s = self.slot_of(v, from);
                self.rt("rt_text_append", &[t, d, s]);
                t
            }
            (Ty::Int(src), Ty::Int(dst)) => {
                let x = if src.bits < 64 {
                    if src.signed { self.b.ins().sextend(types::I64, v) } else { self.b.ins().uextend(types::I64, v) }
                } else {
                    v
                };
                // Faixa conferida no domínio de 64 bits da origem.
                let bad = match (src.signed, dst.signed, dst.bits) {
                    (true, true, 64) | (false, false, 64) => None,
                    (false, true, 64) | (true, false, 64) => Some(self.b.ins().icmp_imm_s(IntCC::SignedLessThan, x, 0)),
                    (true, _, _) => {
                        let lo = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, x, dst.min() as i64);
                        let hi = self.b.ins().icmp_imm_s(IntCC::SignedGreaterThan, x, dst.max() as i64);
                        Some(self.b.ins().bor(lo, hi))
                    }
                    (false, _, _) => Some(self.b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, x, dst.max() as i64)),
                };
                if let Some(bad) = bad {
                    let fail = self.b.create_block();
                    let ok = self.b.create_block();
                    self.b.ins().brif(bad, fail, &[], ok, &[]);
                    self.b.switch_to_block(fail);
                    let ss = self.b.ins().iconst(types::I32, src.signed as i64);
                    let bits = self.b.ins().iconst(types::I32, dst.bits as i64);
                    let ds = self.b.ins().iconst(types::I32, dst.signed as i64);
                    let (lo, hi) = self.span_vals(span);
                    self.rt("rt_fault_convert_int", &[x, ss, bits, ds, lo, hi]);
                    self.b.ins().trap(ir::TrapCode::unwrap_user(2));
                    self.b.switch_to_block(ok);
                }
                if dst.bits < 64 { self.b.ins().ireduce(int_type(*dst), x) } else { x }
            }
            (Ty::Int(src), Ty::Real(b)) => {
                let x = if src.bits < 64 {
                    if src.signed { self.b.ins().sextend(types::I64, v) } else { self.b.ins().uextend(types::I64, v) }
                } else {
                    v
                };
                let f = if src.signed { self.b.ins().fcvt_from_sint(types::F64, x) } else { self.b.ins().fcvt_from_uint(types::F64, x) };
                if *b == 32 { self.b.ins().fdemote(types::F32, f) } else { f }
            }
            (Ty::Real(b), Ty::Int(dst)) => {
                let x = if *b == 32 { self.b.ins().fpromote(types::F64, v) } else { v };
                let t = self.b.ins().trunc(x);
                let lo = self.b.ins().f64const(dst.min() as f64);
                let hi = self.b.ins().f64const((dst.max() + 1) as f64);
                let ge = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, t, lo);
                let lt = self.b.ins().fcmp(FloatCC::LessThan, t, hi);
                let fits = self.b.ins().band(ge, lt);
                let fail = self.b.create_block();
                let ok = self.b.create_block();
                self.b.ins().brif(fits, ok, &[], fail, &[]);
                self.b.switch_to_block(fail);
                let bits = self.b.ins().iconst(types::I32, dst.bits as i64);
                let ds = self.b.ins().iconst(types::I32, dst.signed as i64);
                let (slo, shi) = self.span_vals(span);
                self.rt("rt_fault_convert_real", &[x, bits, ds, slo, shi]);
                self.b.ins().trap(ir::TrapCode::unwrap_user(2));
                self.b.switch_to_block(ok);
                let i = if dst.signed { self.b.ins().fcvt_to_sint(types::I64, t) } else { self.b.ins().fcvt_to_uint(types::I64, t) };
                if dst.bits < 64 { self.b.ins().ireduce(int_type(*dst), i) } else { i }
            }
            (Ty::Real(a), Ty::Real(b)) if a != b => {
                if *b == 32 {
                    self.b.ins().fdemote(types::F32, v)
                } else {
                    self.b.ins().fpromote(types::F64, v)
                }
            }
            _ => v,
        }
    }

    fn unary(&mut self, op: UnOp, v: Value, t: &Ty, span: Span) -> Value {
        match (op, t) {
            (UnOp::Len, _) => {
                let d = self.desc(t);
                let s = self.slot_of(v, t);
                self.rt1("rt_len", &[d, s])
            }
            (UnOp::Neg, Ty::Real(_)) => self.b.ins().fneg(v),
            (UnOp::Neg, Ty::Int(it)) => {
                let z = self.b.ins().iconst(int_type(*it), 0);
                self.arith(BinOp::Sub, z, v, t, t, span)
            }
            (UnOp::Abs, Ty::Real(_)) => self.b.ins().fabs(v),
            (UnOp::Abs, Ty::Int(it)) => {
                if !it.signed {
                    return v;
                }
                let min = self.b.ins().iconst(int_type(*it), it.min() as i64);
                let bad = self.b.ins().icmp(IntCC::Equal, v, min);
                let fail = self.b.create_block();
                let ok = self.b.create_block();
                self.b.ins().brif(bad, fail, &[], ok, &[]);
                self.b.switch_to_block(fail);
                let x = if it.bits == 64 { v } else { self.b.ins().sextend(types::I64, v) };
                let bits = self.b.ins().iconst(types::I32, it.bits as i64);
                let signed = self.b.ins().iconst(types::I32, 1);
                let (lo, hi) = self.span_vals(span);
                self.rt("rt_fault_abs", &[x, bits, signed, lo, hi]);
                self.b.ins().trap(ir::TrapCode::unwrap_user(2));
                self.b.switch_to_block(ok);
                self.b.ins().iabs(v)
            }
            (UnOp::Sqrt, Ty::Real(b)) => {
                let z = if *b == 32 { self.b.ins().f32const(0.0) } else { self.b.ins().f64const(0.0) };
                let neg = self.b.ins().fcmp(FloatCC::LessThan, v, z);
                let fail = self.b.create_block();
                let ok = self.b.create_block();
                self.b.ins().brif(neg, fail, &[], ok, &[]);
                self.b.switch_to_block(fail);
                let x = if *b == 32 { self.b.ins().fpromote(types::F64, v) } else { v };
                let (lo, hi) = self.span_vals(span);
                self.rt("rt_fault_sqrt", &[x, lo, hi]);
                self.b.ins().trap(ir::TrapCode::unwrap_user(2));
                self.b.switch_to_block(ok);
                self.b.ins().sqrt(v)
            }
            (UnOp::BitNot, _) => self.b.ins().bnot(v),
            _ => unreachable!("operação {op:?} em {t:?}"),
        }
    }

    fn arith(&mut self, op: BinOp, a: Value, b: Value, t: &Ty, rhs_ty: &Ty, span: Span) -> Value {
        match t {
            Ty::Real(bits) => {
                let promote = |fx: &mut Self, x: Value| if *bits == 32 { fx.b.ins().fpromote(types::F64, x) } else { x };
                let demote = |fx: &mut Self, x: Value| if *bits == 32 { fx.b.ins().fdemote(types::F32, x) } else { x };
                match op {
                    BinOp::Add => self.b.ins().fadd(a, b),
                    BinOp::Sub => self.b.ins().fsub(a, b),
                    BinOp::Mul => self.b.ins().fmul(a, b),
                    BinOp::Div => self.b.ins().fdiv(a, b),
                    BinOp::Rem | BinOp::Pow => {
                        let (x, y) = (promote(self, a), promote(self, b));
                        let r = self.rt1(if op == BinOp::Rem { "rt_fmod" } else { "rt_pow_real" }, &[x, y]);
                        demote(self, r)
                    }
                    _ => unreachable!(),
                }
            }
            Ty::Int(it) => self.int_arith(op, a, b, *it, rhs_ty, span),
            _ => unreachable!(),
        }
    }

    fn int_arith(&mut self, op: BinOp, a: Value, b: Value, it: IntTy, rhs_ty: &Ty, span: Span) -> Value {
        let t = int_type(it);
        let overflow = textfmt::overflow(it.bits, it.signed);
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                let (r, of) = match (op, it.signed) {
                    (BinOp::Add, true) => self.b.ins().sadd_overflow(a, b),
                    (BinOp::Add, false) => self.b.ins().uadd_overflow(a, b),
                    (BinOp::Sub, true) => self.b.ins().ssub_overflow(a, b),
                    (BinOp::Sub, false) => self.b.ins().usub_overflow(a, b),
                    (BinOp::Mul, true) => self.b.ins().smul_overflow(a, b),
                    _ => self.b.ins().umul_overflow(a, b),
                };
                self.fault_if(of, L0900, &overflow, span);
                r
            }
            BinOp::Div | BinOp::Rem => {
                let zero = self.b.ins().icmp_imm_s(IntCC::Equal, b, 0);
                if op == BinOp::Div {
                    self.fault_if(zero, L0904, textfmt::DIV_ZERO, span);
                } else {
                    self.fault_if(zero, L0905, textfmt::REM_ZERO, span);
                }
                if !it.signed {
                    return if op == BinOp::Div { self.b.ins().udiv(a, b) } else { self.b.ins().urem(a, b) };
                }
                // MIN dividido por -1 não cabe; MIN módulo -1 é 0 (a instrução da máquina falharia).
                let minus_one = self.b.ins().icmp_imm_s(IntCC::Equal, b, -1);
                if op == BinOp::Div {
                    let min = self.b.ins().icmp_imm_s(IntCC::Equal, a, it.min() as i64);
                    let both = self.b.ins().band(minus_one, min);
                    self.fault_if(both, L0900, &overflow, span);
                    self.b.ins().sdiv(a, b)
                } else {
                    let one = self.b.ins().iconst(t, 1);
                    let safe_b = self.b.ins().select(minus_one, one, b);
                    let r = self.b.ins().srem(a, safe_b);
                    let z = self.b.ins().iconst(t, 0);
                    self.b.ins().select(minus_one, z, r)
                }
            }
            BinOp::Pow => {
                let x = if it.bits < 64 {
                    if it.signed { self.b.ins().sextend(types::I64, a) } else { self.b.ins().uextend(types::I64, a) }
                } else {
                    a
                };
                let y = if it.bits < 64 {
                    if it.signed { self.b.ins().sextend(types::I64, b) } else { self.b.ins().uextend(types::I64, b) }
                } else {
                    b
                };
                let bits = self.b.ins().iconst(types::I32, it.bits as i64);
                let signed = self.b.ins().iconst(types::I32, it.signed as i64);
                let (lo, hi) = self.span_vals(span);
                let r = self.rt1("rt_pow_int", &[x, y, bits, signed, lo, hi]);
                if it.bits < 64 { self.b.ins().ireduce(t, r) } else { r }
            }
            BinOp::BitAnd => self.b.ins().band(a, b),
            BinOp::BitOr => self.b.ins().bor(a, b),
            BinOp::BitXor => self.b.ins().bxor(a, b),
            BinOp::Shl | BinOp::Shr => {
                let Ty::Int(yt) = rhs_ty else { unreachable!() };
                // O deslocamento vale de 0 até a largura menos 1.
                let y64 = if yt.bits < 64 {
                    if yt.signed { self.b.ins().sextend(types::I64, b) } else { self.b.ins().uextend(types::I64, b) }
                } else {
                    b
                };
                let cc = if yt.signed { IntCC::SignedGreaterThanOrEqual } else { IntCC::UnsignedGreaterThanOrEqual };
                let too_big = self.b.ins().icmp_imm_s(cc, y64, it.bits as i64);
                let bad = if yt.signed {
                    let neg = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, y64, 0);
                    self.b.ins().bor(neg, too_big)
                } else {
                    too_big
                };
                let fail = self.b.create_block();
                let ok = self.b.create_block();
                self.b.ins().brif(bad, fail, &[], ok, &[]);
                self.b.switch_to_block(fail);
                let bits = self.b.ins().iconst(types::I32, it.bits as i64);
                let signed = self.b.ins().iconst(types::I32, it.signed as i64);
                let (lo, hi) = self.span_vals(span);
                self.rt("rt_fault_shift", &[y64, bits, signed, lo, hi]);
                self.b.ins().trap(ir::TrapCode::unwrap_user(2));
                self.b.switch_to_block(ok);
                if op == BinOp::Shl { self.b.ins().ishl(a, y64) } else { self.b.ins().ushr(a, y64) }
            }
        }
    }
}

fn zero(b: &mut FunctionBuilder, t: types::Type) -> Value {
    match t {
        types::F64 => b.ins().f64const(0.0),
        types::F32 => b.ins().f32const(0.0),
        _ => b.ins().iconst(t, 0),
    }
}

/// Marca os locais passados como confiados (precisam de endereço).
fn mark_addressed(b: &hir::Block, out: &mut [bool]) {
    for s in &b.0 {
        stmt_addressed(s, out);
    }
}

fn stmt_addressed(s: &Stmt, out: &mut [bool]) {
    match &s.kind {
        StmtKind::Let { value, .. } | StmtKind::Write { value, .. } | StmtKind::Expr(value) | StmtKind::Desist(value) => {
            expr_addressed(value, out)
        }
        StmtKind::Assign { value, .. } | StmtKind::Compound { value, .. } | StmtKind::Push { value, .. } => expr_addressed(value, out),
        StmtKind::If { arms, otherwise } => {
            for (c, b) in arms {
                cond_addressed(c, out);
                mark_addressed(b, out);
            }
            if let Some(o) = otherwise {
                mark_addressed(o, out);
            }
        }
        StmtKind::While { cond, body } => {
            cond_addressed(cond, out);
            mark_addressed(body, out);
        }
        StmtKind::Repeat { body, .. } | StmtKind::ForRange { body, .. } | StmtKind::ForEach { body, .. } => mark_addressed(body, out),
        StmtKind::Match { arms, otherwise, .. } => {
            for a in arms {
                mark_addressed(&a.body, out);
            }
            if let Some(o) = otherwise {
                mark_addressed(o, out);
            }
        }
        StmtKind::Return(Some(v)) => expr_addressed(v, out),
        _ => {}
    }
}

fn cond_addressed(c: &Cond, out: &mut [bool]) {
    match c {
        Cond::And(v) | Cond::Or(v) => v.iter().for_each(|c| cond_addressed(c, out)),
        Cond::Not(c) => cond_addressed(c, out),
        Cond::Cmp { lhs, rhs, .. } => {
            expr_addressed(lhs, out);
            expr_addressed(rhs, out);
        }
        Cond::Test(e) | Cond::IsSome(e) | Cond::IsVariant(e, _) => expr_addressed(e, out),
    }
}

fn expr_addressed(e: &hir::Expr, out: &mut [bool]) {
    match &e.kind {
        ExprKind::Call { args, .. } => {
            for a in args {
                if a.mode == ParamMode::Lent
                    && let Some(p) = &a.place
                    && p.proj.is_empty()
                    && let Var::Local(id) = p.root
                {
                    out[id] = true;
                }
                expr_addressed(&a.value, out);
            }
        }
        ExprKind::Try { call, fallback } => {
            expr_addressed(call, out);
            if let Some(f) = fallback {
                expr_addressed(f, out);
            }
        }
        ExprKind::Binary(_, a, b) => {
            expr_addressed(a, out);
            expr_addressed(b, out);
        }
        ExprKind::Unary(_, a) | ExprKind::Copy(a) | ExprKind::Wrap(a) | ExprKind::Unwrap(a) | ExprKind::Convert(a) => {
            expr_addressed(a, out)
        }
        ExprKind::Concat(v) | ExprKind::List(v) | ExprKind::Record(v) | ExprKind::Variant { fields: v, .. } => {
            v.iter().for_each(|x| expr_addressed(x, out))
        }
        ExprKind::Fact(c) => cond_addressed(c, out),
        _ => {}
    }
}
