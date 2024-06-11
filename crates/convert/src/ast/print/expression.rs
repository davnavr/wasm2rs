use crate::ast::{
    print::{self, paths, print_frame},
    Expr,
};

pub(in crate::ast::print) fn print_expression(
    expr: &Expr,
    out: &mut dyn crate::write::Write,
    nested: bool,
    context: &print::Context,
    function: Option<crate::ast::FuncId>,
) {
    macro_rules! nested_expr {
        {$($stmt:stmt;)*} => {{
            if nested {
                out.write_str("(");
            }

            $($stmt)*

            if nested {
                out.write_str(")");
            }
        }};
    }

    match expr {
        Expr::Literal(literal) => literal.print(out),
        Expr::Select {
            val_1,
            val_2,
            condition,
        } => nested_expr! {
            out.write_str("if ");
            condition.print_bool(out, context, function);
            out.write_str(" { ");
            val_1.print(out, false, context, function);
            out.write_str(" } else { ");
            val_2.print(out, false, context, function);
            out.write_str(" }");
        },
        Expr::UnaryOperator { kind, c_1 } => {
            use crate::ast::UnOp;

            macro_rules! rt_math_function {
                ($name:ident @ $offset:ident) => {{
                    out.write_str(paths::RT_MATH);
                    out.write_str(concat!("::", stringify!($name), "("));
                    c_1.print(out, true, context, function);
                    out.write_str(")");
                    print::print_trap_with(out, function, *$offset, context.debug_info);
                    out.write_str("?");
                }};
            }

            macro_rules! simple_cast {
                ($to:ident) => {
                    nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(concat!(" as ", stringify!($to)));
                    }
                };
            }

            macro_rules! double_cast {
                ($start:ident as $end:ident) => {
                    nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(concat!(
                            " as ",
                            stringify!($start),
                            " as ",
                            stringify!($end)
                        ));
                    }
                };
            }

            macro_rules! primitive_method {
                ($name:ident) => {{
                    c_1.print(out, true, context, function);
                    out.write_str(concat!(".", stringify!($name), "()"));
                }};
            }

            match kind {
                UnOp::IxxEqz => nested_expr! {
                    out.write_str("(");
                    c_1.print(out, false, context, function);
                    out.write_str(" == 0) as i32");
                },
                UnOp::I32Clz => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".leading_zeros() as i32");
                },
                UnOp::I64Clz => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".leading_zeros() as i64");
                },
                UnOp::I32Ctz => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".trailing_zeros() as i32");
                },
                UnOp::I64Ctz => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".trailing_zeros() as i64");
                },
                UnOp::I32Popcnt => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".count_ones() as i32");
                },
                UnOp::I64Popcnt => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(".count_ones() as i64");
                },
                UnOp::FxxAbs => primitive_method!(abs),
                UnOp::FxxNeg => nested_expr! {
                    // `::core::ops::Neg` on `f32` and `f64` do the same operation in Rust.
                    out.write_str("-");
                    c_1.print(out, true, context, function);
                },
                UnOp::FxxCeil => primitive_method!(ceil),
                UnOp::FxxFloor => primitive_method!(floor),
                UnOp::FxxTrunc => primitive_method!(trunc),
                UnOp::FxxNearest => primitive_method!(round_ties_even),
                UnOp::FxxSqrt => primitive_method!(sqrt),
                UnOp::I32WrapI64 | UnOp::I32TruncSatFxxS => simple_cast!(i32),
                UnOp::I32TruncF32S { offset } => rt_math_function!(i32_trunc_f32_s @ offset),
                UnOp::I32TruncF32U { offset } => rt_math_function!(i32_trunc_f32_u @ offset),
                UnOp::I32TruncF64S { offset } => rt_math_function!(i32_trunc_f64_s @ offset),
                UnOp::I32TruncF64U { offset } => rt_math_function!(i32_trunc_f64_u @ offset),
                UnOp::I64ExtendI32S | UnOp::I64TruncSatFxxS => simple_cast!(i64),
                UnOp::I64ExtendI32U => double_cast!(u32 as i64),
                UnOp::I64TruncF32S { offset } => rt_math_function!(i64_trunc_f32_s @ offset),
                UnOp::I64TruncF32U { offset } => rt_math_function!(i64_trunc_f32_u @ offset),
                UnOp::I64TruncF64S { offset } => rt_math_function!(i64_trunc_f64_s @ offset),
                UnOp::I64TruncF64U { offset } => rt_math_function!(i64_trunc_f64_u @ offset),
                UnOp::F32ConvertIxxS => nested_expr! {
                    // - Rust uses "roundTiesToEven".
                    // - WebAssembly specifies round-to-nearest ties-to-even.
                    //
                    // Are they the same?
                    //
                    // Rust: https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast
                    // WASM: https://webassembly.github.io/spec/core/exec/numerics.html#rounding
                    simple_cast!(f32);
                },
                UnOp::F32ConvertI32U => double_cast!(u32 as f32),
                UnOp::F32ConvertI64U => double_cast!(u64 as f32),
                UnOp::F32DemoteF64 => nested_expr! {
                    // TODO: Does Rust's conversion of `f64` to `f32` preserve the "canonical NaN"
                    out.write_str("/* f32.demote_f64 */ ");
                    c_1.print(out, true, context, function);
                    out.write_str(" as f32");
                },
                UnOp::F64ConvertIxxS => simple_cast!(f64),
                UnOp::F64ConvertI32U => double_cast!(u32 as f64),
                UnOp::F64ConvertI64U => double_cast!(u64 as f64),
                UnOp::F64PromoteF32 => nested_expr! {
                    // TODO: Does Rust's conversion of `f32` to `f64` preserve the "canonical NaN"
                    out.write_str("/* f64.promote_f32 */ ");
                    simple_cast!(f64);
                },
                UnOp::I32ReinterpretF32 => nested_expr! {
                    out.write_str("f32::to_bits(");
                    c_1.print(out, false, context, function);
                    out.write_str(") as i32");
                },
                UnOp::I64ReinterpretF64 => nested_expr! {
                    out.write_str("f64::to_bits(");
                    c_1.print(out, false, context, function);
                    out.write_str(") as i64");
                },
                UnOp::F32ReinterpretI32 => {
                    out.write_str("f32::from_bits(");
                    c_1.print(out, true, context, function);
                    out.write_str(" as u32)");
                }
                UnOp::F64ReinterpretI64 => {
                    out.write_str("f64::from_bits(");
                    c_1.print(out, true, context, function);
                    out.write_str(" as u64)");
                }
                UnOp::I32Extend8S => double_cast!(i8 as i32),
                UnOp::I32Extend16S => double_cast!(i16 as i32),
                UnOp::I64Extend8S => double_cast!(i8 as i64),
                UnOp::I64Extend16S => double_cast!(i16 as i64),
                UnOp::I64Extend32S => double_cast!(i32 as i64),
                // Float-to-integer saturation operations translate exactly to Rust casts.
                UnOp::I32TruncSatFxxU => double_cast!(u32 as i32),
                UnOp::I64TruncSatFxxU => double_cast!(u64 as i64),
            }
        }
        Expr::BinaryOperator { kind, c_1, c_2 } => {
            use crate::ast::BinOp;

            macro_rules! infix_operator {
                ($operator:literal) => {
                    nested_expr! {
                        c_1.print(out, true, context, function);
                        out.write_str(concat!(" ", $operator, " "));
                        c_2.print(out, true, context, function);
                    }
                };
            }

            macro_rules! infix_comparison {
                ($operator:literal $(as $cast:ident)?) => {
                    nested_expr! {
                        out.write_str("((");
                        c_1.print(out, true, context, function);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            ") ",
                            $operator,
                            " (",
                        ));
                        c_2.print(out, true, context, function);
                        out.write_str(concat!(
                            $(" as ", stringify!($cast),)?
                            ")) as i32"
                        ));
                    }
                };
            }

            macro_rules! function {
                ($($name:expr),+) => {{
                    $(out.write_str($name);)+
                    out.write_str("(");
                    c_1.print(out, false, context, function);
                    out.write_str(", ");
                    c_2.print(out, false, context, function);
                    out.write_str(")");
                }};
            }

            macro_rules! rt_math_function {
                ($name:ident @ $offset:ident) => {{
                    function!(paths::RT_MATH, concat!("::", stringify!($name)));
                    print::print_trap_with(out, function, $offset, context.debug_info);
                    out.write_str("?");
                }};
            }

            match *kind {
                BinOp::Eq => infix_comparison!("=="),
                BinOp::Ne => infix_comparison!("!="),
                BinOp::IxxLtS | BinOp::FxxLt => infix_comparison!("<"),
                BinOp::IxxGtS | BinOp::FxxGt => infix_comparison!(">"),
                BinOp::I32LtU => infix_comparison!("<" as u32),
                BinOp::I32GtU => infix_comparison!(">" as u32),
                BinOp::I64LtU => infix_comparison!("<" as u64),
                BinOp::I64GtU => infix_comparison!(">" as u64),
                BinOp::IxxLeS | BinOp::FxxLe => infix_comparison!("<="),
                BinOp::IxxGeS | BinOp::FxxGe => infix_comparison!(">="),
                BinOp::I32LeU => infix_comparison!("<=" as u32),
                BinOp::I32GeU => infix_comparison!(">=" as u32),
                BinOp::I64LeU => infix_comparison!("<=" as u64),
                BinOp::I64GeU => infix_comparison!(">=" as u64),
                BinOp::FxxAdd => infix_operator!("+"),
                BinOp::FxxSub => infix_operator!("-"),
                BinOp::FxxMul => infix_operator!("*"),
                BinOp::FxxDiv => infix_operator!("/"),
                BinOp::F32Min => function!(paths::RT_MATH, "::f32_min"),
                BinOp::F32Max => function!(paths::RT_MATH, "::f32_max"),
                BinOp::F64Min => function!(paths::RT_MATH, "::f64_min"),
                BinOp::F64Max => function!(paths::RT_MATH, "::f64_max"),
                BinOp::FxxCopysign => {
                    c_1.print(out, true, context, function);
                    out.write_str(".copysign(");
                    c_2.print(out, false, context, function);
                    out.write_str(")");
                }
                BinOp::I32Add => function!("i32::wrapping_add"),
                BinOp::I64Add => function!("i64::wrapping_add"),
                BinOp::I32Sub => function!("i32::wrapping_sub"),
                BinOp::I64Sub => function!("i64::wrapping_sub"),
                BinOp::I32Mul => function!("i32::wrapping_mul"),
                BinOp::I64Mul => function!("i64::wrapping_mul"),
                BinOp::I32DivS { offset } => rt_math_function!(i32_div_s @ offset),
                BinOp::I64DivS { offset } => rt_math_function!(i64_div_s @ offset),
                BinOp::I32DivU { offset } => rt_math_function!(i32_div_u @ offset),
                BinOp::I64DivU { offset } => rt_math_function!(i64_div_u @ offset),
                BinOp::I32RemS { offset } => rt_math_function!(i32_rem_s @ offset),
                BinOp::I64RemS { offset } => rt_math_function!(i64_rem_s @ offset),
                BinOp::I32RemU { offset } => rt_math_function!(i32_rem_u @ offset),
                BinOp::I64RemU { offset } => rt_math_function!(i64_rem_u @ offset),
                BinOp::IxxAnd => infix_operator!("&"),
                BinOp::IxxOr => infix_operator!("|"),
                BinOp::IxxXor => infix_operator!("^"),
                BinOp::I32Shl => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(" << (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u32 % 32)");
                },
                BinOp::I64Shl => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(" << (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u64 % 64)");
                },
                BinOp::I32ShrS => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(" >> (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u32 % 32)");
                },
                BinOp::I64ShrS => nested_expr! {
                    c_1.print(out, true, context, function);
                    out.write_str(" >> (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u64 % 64)");
                },
                BinOp::I32ShrU => nested_expr! {
                    out.write_str("(");
                    c_1.print(out, true, context, function);
                    out.write_str(" as u32 >> (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u32 % 32)) as i32");
                },
                BinOp::I64ShrU => nested_expr! {
                    out.write_str("(");
                    c_1.print(out, true, context, function);
                    out.write_str(" as u64 >> (");
                    c_2.print(out, true, context, function);
                    out.write_str(" as u64 % 64)) as i64");
                },
                BinOp::I32Rotl => {
                    c_1.print(out, true, context, function);
                    out.write_str(".rotate_left((");
                    c_2.print(out, true, context, function);
                    out.write_str(" % 32) as u32)");
                }
                BinOp::I64Rotl => {
                    c_1.print(out, true, context, function);
                    out.write_str(".rotate_left((");
                    c_2.print(out, true, context, function);
                    out.write_str(" % 64) as u32)");
                }
                BinOp::I32Rotr => {
                    c_1.print(out, true, context, function);
                    out.write_str(".rotate_right((");
                    c_2.print(out, true, context, function);
                    out.write_str(" % 32) as u32)");
                }
                BinOp::I64Rotr => {
                    c_1.print(out, true, context, function);
                    out.write_str(".rotate_right((");
                    c_2.print(out, true, context, function);
                    out.write_str(" % 64) as u32)");
                }
            }
        }
        Expr::RefIsNull(reference) => {
            out.write_str("(");
            reference.print(out, false, context, function);
            out.write_str("embedder::rt::table::NullableTableElement::NULL) as i32");
        }
        // TODO: GetLocal, Temporary, and LoopInput need to indicate if clone() is required.
        Expr::GetLocal(local) => write!(out, "{local}"),
        Expr::GetGlobal(global) => match context.wasm.global_kind(*global) {
            crate::context::GlobalKind::Const => write!(out, "Self::{global:#}"),
            crate::context::GlobalKind::ImmutableField => write!(out, "self.{global}"),
            crate::context::GlobalKind::MutableField { import: None } => {
                write!(out, "self.{global}.get()")
            }
            crate::context::GlobalKind::MutableField {
                import: Some(import),
            } => todo!("printing of mutable global imports {import:?}"),
        },
        Expr::Temporary(temp) => write!(out, "{temp}"),
        Expr::LoopInput(input) => write!(out, "{input}"),
        Expr::TableGet {
            table,
            index,
            instruction_offset,
        } => {
            out.write_str(paths::RT_TABLE);
            write!(
                out,
                "::get::<{}, embedder::{table}, embedder::Trap>(",
                table.0
            );
            print::print_table(out, *table, context.wasm);
            out.write_str(", ");
            index.print(out, false, context, function);
            out.write_str(", ");
            print_frame(out, function, *instruction_offset, context.debug_info);
            out.write_str(")?");
        }
        Expr::TableSize(table) => {
            out.write_str(paths::RT_TABLE);
            out.write_str("::size(");
            print::print_table(out, *table, context.wasm);
            write!(out, ")?");
        }
        Expr::TableGrow { table, delta } => {
            write!(
                out,
                "{}::grow(&self.{}, ",
                paths::RT_TABLE,
                context.wasm.table_ident(*table)
            );
            delta.print(out, false, context, function);
            out.write_str(")");
        }
        Expr::MemoryLoad {
            memory,
            kind,
            address,
            offset,
            instruction_offset,
        } => {
            use crate::ast::{I32StorageSize, I64StorageSize, LoadKind, SignExtensionMode};

            match kind {
                LoadKind::F32 => out.write_str("f32::from_bits("),
                LoadKind::F64 => out.write_str("f64::from_bits("),
                LoadKind::AsI32 { .. } | LoadKind::AsI64 { .. } if nested => out.write_str("("),
                _ => (),
            }

            out.write_str(paths::RT_MEM);
            out.write_str("::");
            out.write_str(match kind {
                LoadKind::AsI32 {
                    storage_size: I32StorageSize::I8,
                    ..
                }
                | LoadKind::AsI64 {
                    storage_size: I64StorageSize::I8,
                    ..
                } => "i8",
                LoadKind::AsI32 {
                    storage_size: I32StorageSize::I16,
                    ..
                }
                | LoadKind::AsI64 {
                    storage_size: I64StorageSize::I16,
                    ..
                } => "i16",
                LoadKind::I32
                | LoadKind::F32
                | LoadKind::AsI64 {
                    storage_size: I64StorageSize::I32,
                    ..
                } => "i32",
                LoadKind::I64 | LoadKind::F64 => "i64",
            });

            let memory64 = context.wasm.types.memory_at(memory.0).memory64;
            write!(out, "_load::<{}, ", memory.0);
            out.write_str(if memory64 { "u64" } else { "u32" });
            write!(out, ", embedder::{memory}, embedder::Trap>(");
            let memory_ident = context.wasm.memory_ident(*memory);

            if matches!(memory_ident, crate::context::MemoryIdent::Id(_)) {
                out.write_str("&");
            }

            write!(out, "self.{memory_ident}, {offset}, ");
            address.print(out, false, context, function);

            out.write_str(", ");
            print_frame(out, function, *instruction_offset, context.debug_info);
            out.write_str(")?");

            match kind {
                LoadKind::F32 => out.write_str(" as u32)"),
                LoadKind::F64 => out.write_str(" as u64)"),
                LoadKind::AsI32 {
                    storage_size,
                    sign_extension,
                } => {
                    if matches!(sign_extension, SignExtensionMode::Unsigned) {
                        out.write_str(match storage_size {
                            crate::ast::I32StorageSize::I8 => " as u8",
                            crate::ast::I32StorageSize::I16 => " as u16",
                        });
                    }

                    out.write_str(" as i32");

                    if nested {
                        out.write_str(")");
                    }
                }
                LoadKind::AsI64 {
                    storage_size,
                    sign_extension,
                } => {
                    if matches!(sign_extension, SignExtensionMode::Unsigned) {
                        out.write_str(match storage_size {
                            crate::ast::I64StorageSize::I8 => " as u8",
                            crate::ast::I64StorageSize::I16 => " as u16",
                            crate::ast::I64StorageSize::I32 => " as u32",
                        });
                    }

                    out.write_str(" as i64");

                    if nested {
                        out.write_str(")");
                    }
                }
                _ => (),
            }
        }
        Expr::MemorySize(memory) => {
            write!(
                out,
                "{}::size(&self.{})",
                paths::RT_MEM,
                context.wasm.memory_ident(*memory)
            );
        }
        Expr::MemoryGrow { memory, delta } => {
            write!(
                out,
                "{}::grow(&self.{}, ",
                paths::RT_MEM,
                context.wasm.memory_ident(*memory)
            );
            delta.print(out, false, context, function);
            out.write_str(")");
        }
        Expr::Call {
            callee,
            arguments,
            offset,
        } => {
            debug_assert_eq!(
                context.wasm.types[context.wasm.types.core_function_at(callee.0)]
                    .unwrap_func()
                    .results()
                    .len(),
                1,
                "use Statement::Call instead"
            );

            print::print_call_expr(out, *callee, *arguments, context, function, *offset)
        }
        Expr::CallIndirect {
            result_type,
            table,
            callee,
            arguments,
            offset,
        } => {
            out.write_str(paths::RT_FUNC_REF);
            write!(out, "::call_indirect_{}::<{}", arguments.len(), table.0);

            for _ in 0..arguments.len() {
                out.write_str(", _");
            }

            write!(out, ", {result_type}, _, _>(");
            print::print_table(out, *table, context.wasm);
            out.write_str(", ");
            callee.print(out, false, context, function);

            for arg in context.arena.get_list(*arguments) {
                out.write_str(", ");
                arg.print(out, false, context, function);
            }

            out.write_str(", ");
            print_frame(out, function, *offset, context.debug_info);
            out.write_str(")?");
        }
    }
}
