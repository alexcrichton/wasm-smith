mod code_builder;
mod encode;
mod terminate;

use crate::code_builder::CodeBuilderAllocations;
use arbitrary::{Arbitrary, Result, Unstructured};
use std::collections::HashSet;

/// A pseudo-random WebAssembly module.
///
/// Construct instances of this type with [the `Arbitrary`
/// trait](https://docs.rs/arbitrary/*/arbitrary/trait.Arbitrary.html).
#[derive(Debug, Default)]
pub struct Module {
    types: Vec<FuncType>,
    imports: Vec<(String, String, Import)>,
    funcs: Vec<u32>,
    table: Option<TableType>,
    memory: Option<MemoryType>,
    globals: Vec<Global>,
    exports: Vec<(String, Export)>,
    start: Option<u32>,
    elems: Vec<ElementSegment>,
    code: Vec<Code>,
    data: Vec<DataSegment>,
}

impl Arbitrary for Module {
    fn arbitrary(u: &mut Unstructured) -> Result<Self> {
        let mut module = Module::default();
        module.types = u.arbitrary()?;
        module.arbitrary_imports(u)?;
        module.arbitrary_funcs(u)?;
        if module.table_imports() == 0 {
            module.table = u.arbitrary()?;
        }
        if module.memory_imports() == 0 {
            module.memory = u.arbitrary()?;
        }
        module.arbitrary_globals(u)?;
        module.arbitrary_exports(u)?;
        module.arbitrary_start(u)?;
        module.arbitrary_elems(u)?;
        module.arbitrary_code(u)?;
        module.arbitrary_data(u)?;
        Ok(module)
    }
}

#[derive(Clone, Debug)]
struct FuncType {
    params: Vec<ValType>,
    results: Vec<ValType>,
}

impl Arbitrary for FuncType {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        const MAX_PARAMS: usize = 20;
        let mut params = vec![];
        loop {
            let keep_going = params.len() < MAX_PARAMS && u.arbitrary().unwrap_or(false);
            if !keep_going {
                break;
            }

            params.push(u.arbitrary()?);
        }

        const MAX_RESULTS: usize = 20;
        let mut results = vec![];
        loop {
            let keep_going = params.len() < MAX_RESULTS && u.arbitrary().unwrap_or(false);
            if !keep_going {
                break;
            }

            results.push(u.arbitrary()?);
        }

        Ok(FuncType { params, results })
    }
}

#[derive(Arbitrary, Clone, Copy, Debug, PartialEq, Eq)]
enum ValType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Clone, Debug)]
enum Import {
    Func(u32),
    Table(TableType),
    Memory(MemoryType),
    Global(GlobalType),
}

#[derive(Arbitrary, Clone, Debug)]
struct TableType {
    limits: Limits,
}

#[derive(Clone, Debug)]
struct MemoryType {
    limits: Limits,
}

impl Arbitrary for MemoryType {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        let min = u.int_in_range(0..=65536)?;
        let max = if u.arbitrary().unwrap_or(false) {
            Some(if min == 65536 {
                65536
            } else {
                u.int_in_range(min..=65536)?
            })
        } else {
            None
        };
        Ok(MemoryType {
            limits: Limits { min, max },
        })
    }
}

#[derive(Clone, Debug)]
struct Limits {
    min: u32,
    max: Option<u32>,
}

impl Arbitrary for Limits {
    fn arbitrary(u: &mut Unstructured) -> Result<Self> {
        if u.arbitrary()? {
            let (a, b) = u.arbitrary()?;
            if a <= b {
                Ok(Limits {
                    min: a,
                    max: Some(b),
                })
            } else {
                Ok(Limits {
                    min: b,
                    max: Some(a),
                })
            }
        } else {
            Ok(Limits {
                min: u.arbitrary()?,
                max: None,
            })
        }
    }
}

#[derive(Clone, Debug)]
struct Global {
    ty: GlobalType,
    expr: Instruction,
}

#[derive(Arbitrary, Clone, Debug)]
struct GlobalType {
    val_type: ValType,
    mutable: bool,
}

#[derive(Clone, Debug)]
enum Export {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

#[derive(Debug)]
struct ElementSegment {
    // table_index: 0,
    offset: Instruction,
    init: Vec<u32>,
}

#[derive(Debug)]
struct Code {
    locals: Vec<ValType>,
    instructions: Vec<Instruction>,
}

#[derive(Clone, Copy, Debug)]
enum BlockType {
    Empty,
    Result(ValType),
    FuncType(u32),
}

impl BlockType {
    fn params_results(&self, module: &Module) -> (Vec<ValType>, Vec<ValType>) {
        match self {
            BlockType::Empty => (vec![], vec![]),
            BlockType::Result(t) => (vec![], vec![*t]),
            BlockType::FuncType(ty) => {
                let ty = &module.types[*ty as usize];
                (ty.params.clone(), ty.results.clone())
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct MemArg {
    offset: u32,
    align: u32,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
enum Instruction {
    // Control instructions.
    Unreachable,
    Nop,
    Block(BlockType),
    Loop(BlockType),
    If(BlockType),
    Else,
    End,
    Br(u32),
    BrIf(u32),
    BrTable(Vec<u32>, u32),
    Return,
    Call(u32),
    CallIndirect(u32),

    // Parametric instructions.
    Drop,
    Select,

    // Variable instructions.
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Memory instructions.
    I32Load(MemArg),
    I64Load(MemArg),
    F32Load(MemArg),
    F64Load(MemArg),
    I32Load8_S(MemArg),
    I32Load8_U(MemArg),
    I32Load16_S(MemArg),
    I32Load16_U(MemArg),
    I64Load8_S(MemArg),
    I64Load8_U(MemArg),
    I64Load16_S(MemArg),
    I64Load16_U(MemArg),
    I64Load32_S(MemArg),
    I64Load32_U(MemArg),
    I32Store(MemArg),
    I64Store(MemArg),
    F32Store(MemArg),
    F64Store(MemArg),
    I32Store8(MemArg),
    I32Store16(MemArg),
    I64Store8(MemArg),
    I64Store16(MemArg),
    I64Store32(MemArg),
    MemorySize,
    MemoryGrow,

    // Numeric instructions.
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Eqz,
    I32Eq,
    I32Neq,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I64Eqz,
    I64Eq,
    I64Neq,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    F32Eq,
    F32Neq,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F64Eq,
    F64Neq,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
    I32TruncSatF32S,
    I32TruncSatF32U,
    I32TruncSatF64S,
    I32TruncSatF64U,
    I64TruncSatF32S,
    I64TruncSatF32U,
    I64TruncSatF64S,
    I64TruncSatF64U,
}

#[derive(Debug)]
struct DataSegment {
    // `memory_index: u32` is currently always 0.
    offset: Instruction,
    init: Vec<u8>,
}

impl Module {
    fn arbitrary_imports(&mut self, u: &mut Unstructured) -> Result<()> {
        let mut choices: Vec<fn(&mut Unstructured, &mut Module) -> Result<Import>> =
            Vec::with_capacity(4);

        if !self.types.is_empty() {
            choices.push(|u, m| {
                let max = m.types.len() as u32 - 1;
                Ok(Import::Func(u.int_in_range(0..=max)?))
            });
        }
        choices.push(|u, _| Ok(Import::Global(u.arbitrary()?)));

        let num_stable_choices = choices.len();
        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            choices.truncate(num_stable_choices);
            if self.memory_imports() == 0 {
                choices.push(|u, _| Ok(Import::Memory(u.arbitrary()?)));
            }
            if self.table_imports() == 0 {
                choices.push(|u, _| Ok(Import::Table(u.arbitrary()?)));
            }

            let module = u.arbitrary()?;
            let name = u.arbitrary()?;

            let f = u.choose(&choices)?;
            let import = f(u, self)?;
            if let Import::Memory(_) = &import {
                // Remove the memory import choice, since we don't support
                // multiple memories.
                choices.pop();
            }

            self.imports.push((module, name, import));
        }
    }

    fn funcs<'a>(&'a self) -> impl Iterator<Item = (u32, &'a FuncType)> + 'a {
        self.imports
            .iter()
            .filter_map(|(_, _, imp)| match imp {
                Import::Func(ty) => Some(*ty),
                _ => None,
            })
            .chain(self.funcs.iter().cloned())
            .map(move |ty| &self.types[ty as usize])
            .enumerate()
            .map(|(f, ty)| (f as u32, ty))
    }

    fn func_imports(&self) -> u32 {
        self.imports
            .iter()
            .filter(|imp| matches!(imp, (_, _, Import::Func(_))))
            .count() as u32
    }

    fn table_imports(&self) -> u32 {
        self.imports
            .iter()
            .filter(|imp| matches!(imp, (_, _, Import::Table(_))))
            .count() as u32
    }

    fn memory_imports(&self) -> u32 {
        self.imports
            .iter()
            .filter(|imp| matches!(imp, (_, _, Import::Memory(_))))
            .count() as u32
    }

    fn global_imports(&self) -> u32 {
        self.imports
            .iter()
            .filter(|imp| matches!(imp, (_, _, Import::Global(_))))
            .count() as u32
    }

    fn arbitrary_funcs(&mut self, u: &mut Unstructured) -> Result<()> {
        if self.types.is_empty() {
            return Ok(());
        }

        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            let max = self.types.len() as u32 - 1;
            let ty = u.int_in_range(0..=max)?;
            self.funcs.push(ty);
        }
    }

    fn arbitrary_globals(&mut self, u: &mut Unstructured) -> Result<()> {
        let mut choices: Vec<Box<dyn Fn(&mut Unstructured, ValType) -> Result<Instruction>>> =
            vec![];

        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            let ty = u.arbitrary::<GlobalType>()?;

            choices.clear();
            choices.push(Box::new(|u, ty| {
                Ok(match ty {
                    ValType::I32 => Instruction::I32Const(u.arbitrary()?),
                    ValType::I64 => Instruction::I64Const(u.arbitrary()?),
                    ValType::F32 => Instruction::F32Const(u.arbitrary()?),
                    ValType::F64 => Instruction::F64Const(u.arbitrary()?),
                })
            }));

            let mut global_idx = 0;
            for (_, _, imp) in &self.imports {
                match imp {
                    Import::Global(g) => {
                        if g.val_type == ty.val_type {
                            choices
                                .push(Box::new(move |_, _| Ok(Instruction::GlobalGet(global_idx))));
                        }
                        global_idx += 1;
                    }
                    _ => {}
                }
            }

            let f = u.choose(&choices)?;
            let expr = f(u, ty.val_type)?;
            self.globals.push(Global { ty, expr });
        }
    }

    fn arbitrary_exports(&mut self, u: &mut Unstructured) -> Result<()> {
        let mut choices: Vec<fn(&mut Unstructured, &mut Module) -> Result<Export>> =
            Vec::with_capacity(4);

        if !self.funcs.is_empty() {
            choices.push(|u, m| {
                let max = m.func_imports() + m.funcs.len() as u32 - 1;
                let idx = u.int_in_range(0..=max)?;
                Ok(Export::Func(idx))
            });
        }

        if self.table.is_some() {
            choices.push(|_, _| Ok(Export::Table(0)));
        }

        if self.memory.is_some() {
            choices.push(|_, _| Ok(Export::Memory(0)));
        }

        if !self.globals.is_empty() {
            choices.push(|u, m| {
                let max = m.global_imports() + m.globals.len() as u32 - 1;
                let idx = u.int_in_range(0..=max)?;
                Ok(Export::Global(idx))
            });
        }

        if choices.is_empty() {
            return Ok(());
        }

        let mut export_names = HashSet::new();
        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            let mut name = u.arbitrary::<String>()?;
            while export_names.contains(&name) {
                name.push_str(&format!("{}", export_names.len()));
            }
            export_names.insert(name.clone());

            let f = u.choose(&choices)?;
            let export = f(u, self)?;
            self.exports.push((name, export));
        }
    }

    fn arbitrary_start(&mut self, u: &mut Unstructured) -> Result<()> {
        let mut choices = Vec::with_capacity(self.func_imports() as usize + self.funcs.len());
        let mut func_index = 0;

        for (_, _, imp) in &self.imports {
            if let Import::Func(ty) = imp {
                let ty = &self.types[*ty as usize];
                if ty.params.is_empty() && ty.results.is_empty() {
                    choices.push(func_index as u32);
                }
                func_index += 1;
            }
        }

        for ty in &self.funcs {
            let ty = &self.types[*ty as usize];
            if ty.params.is_empty() && ty.results.is_empty() {
                choices.push(func_index as u32);
            }
            func_index += 1;
        }

        if !choices.is_empty() && u.arbitrary().unwrap_or(false) {
            let f = *u.choose(&choices)?;
            self.start = Some(f);
        }

        Ok(())
    }

    fn arbitrary_elems(&mut self, u: &mut Unstructured) -> Result<()> {
        if (self.table.is_none() && self.table_imports() == 0)
            || (self.funcs.is_empty() && self.func_imports() == 0)
        {
            return Ok(());
        }

        let func_max = self.func_imports() + self.funcs.len() as u32 - 1;

        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            let mut offset_global_choices = vec![];
            let mut global_index = 0;
            for (_, _, imp) in &self.imports {
                if let Import::Global(g) = imp {
                    if !g.mutable && g.val_type == ValType::I32 {
                        offset_global_choices.push(global_index);
                    }
                    global_index += 1;
                }
            }
            let offset = if !offset_global_choices.is_empty() && u.arbitrary()? {
                let g = u.choose(&offset_global_choices)?;
                Instruction::GlobalGet(*g)
            } else {
                Instruction::I32Const(u.arbitrary()?)
            };

            let mut init = vec![];
            loop {
                let keep_going = u.arbitrary().unwrap_or(false);
                if !keep_going {
                    break;
                }

                let func_idx = u.int_in_range(0..=func_max)?;
                init.push(func_idx);
            }

            self.elems.push(ElementSegment { offset, init });
        }
    }

    fn arbitrary_code(&mut self, u: &mut Unstructured) -> Result<()> {
        self.code.reserve(self.funcs.len());
        let mut allocs = CodeBuilderAllocations::default();
        for ty in &self.funcs {
            let ty = &self.types[*ty as usize];
            let body = self.arbitrary_func_body(u, ty, &mut allocs)?;
            self.code.push(body);
        }
        Ok(())
    }

    fn arbitrary_func_body(
        &self,
        u: &mut Unstructured,
        ty: &FuncType,
        allocs: &mut CodeBuilderAllocations,
    ) -> Result<Code> {
        let locals = self.arbitrary_locals(u)?;
        let builder = allocs.builder(ty, &locals);
        let instructions = builder.arbitrary(u, self)?;

        Ok(Code {
            locals,
            instructions,
        })
    }

    fn arbitrary_locals(&self, u: &mut Unstructured) -> Result<Vec<ValType>> {
        const MAX_LOCALS: usize = 100;
        let mut locals = vec![];
        loop {
            let keep_going = locals.len() < MAX_LOCALS && u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(locals);
            }

            locals.push(u.arbitrary()?);
        }
    }

    fn arbitrary_data(&mut self, u: &mut Unstructured) -> Result<()> {
        if self.memory.is_none() && self.memory_imports() == 0 {
            return Ok(());
        }

        let mut choices: Vec<Box<dyn Fn(&mut Unstructured) -> Result<Instruction>>> = vec![];

        loop {
            let keep_going = u.arbitrary().unwrap_or(false);
            if !keep_going {
                return Ok(());
            }

            if choices.is_empty() {
                choices.push(Box::new(|u| Ok(Instruction::I32Const(u.arbitrary()?))));

                let mut global_idx = 0;
                for (_, _, imp) in &self.imports {
                    match imp {
                        Import::Global(g) => {
                            if !g.mutable && g.val_type == ValType::I32 {
                                choices.push(Box::new(move |_| {
                                    Ok(Instruction::GlobalGet(global_idx))
                                }));
                            }
                            global_idx += 1;
                        }
                        _ => {}
                    }
                }
            }

            let f = u.choose(&choices)?;
            let offset = f(u)?;
            let init = u.arbitrary()?;
            self.data.push(DataSegment { offset, init });
        }
    }
}
