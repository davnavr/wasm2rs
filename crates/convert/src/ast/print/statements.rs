use crate::ast::print::{self, paths, print_frame, print_indentation, Indentation};

fn print_loop_inputs(
    out: &mut dyn crate::write::Write,
    context: &print::Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    indent_level: u32,
    r#loop: crate::ast::BlockId,
    values: crate::ast::ExprListId,
) {
    for (expr, number) in context.arena.get_list(values).iter().zip(0u32..=u32::MAX) {
        write!(out, "{} = ", crate::ast::LoopInput { r#loop, number });

        expr.print(out, false, context, Some(function));
        out.write_str(";\n");
        print_indentation(out, indentation, indent_level);
    }
}

#[derive(Clone, Copy, Debug)]
struct BranchTableCase {
    target: crate::ast::BranchTarget,
    values: crate::ast::ExprListId,
}

// Contains code duplicated with cases for `Statement::Branch` in `print_statements()`
fn print_branch_table_case(
    out: &mut dyn crate::write::Write,
    context: &print::Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    indent_level: u32,
    is_last: bool,
    case: BranchTableCase,
) {
    match case.target {
        crate::ast::BranchTarget::Return => {
            let can_unwind = context
                .wasm
                .function_attributes
                .unwind_kind(function)
                .can_unwind();

            if !is_last {
                out.write_str("return ");

                if !case.values.is_empty() || can_unwind {
                    out.write_str(" ");
                }
            }

            if can_unwind {
                out.write_str("Ok(");
            }

            case.values.print(
                out,
                case.values.len() > 1 || (case.values.is_empty() && can_unwind),
                context,
                Some(function),
            );

            if can_unwind {
                out.write_str(")");
            }
        }
        crate::ast::BranchTarget::Block(block) => {
            write!(out, "break {block}");

            if !case.values.is_empty() {
                out.write_str(" ");

                case.values
                    .print(out, case.values.len() > 1, context, Some(function));
            }
        }
        crate::ast::BranchTarget::Loop(r#loop) => {
            if !case.values.is_empty() {
                out.write_str("{\n");
                print_indentation(out, indentation, indent_level + 1);

                print_loop_inputs(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level + 1,
                    r#loop,
                    case.values,
                );
            }

            write!(out, "continue {}", r#loop);

            if !case.values.is_empty() {
                out.write_str("\n");
                print_indentation(out, indentation, indent_level);
                out.write_str("}");
            }
        }
    }

    out.write_str(",\n");
}

fn print_call_statement_results(
    out: &mut dyn crate::write::Write,
    results: Option<(crate::ast::TempId, std::num::NonZeroU32)>,
) {
    if let Some((results, result_count)) = results {
        out.write_str("let ");

        if result_count.get() > 1 {
            out.write_str("(");
        }

        for i in 0..result_count.get() {
            if i > 0 {
                out.write_str(", ");
            }

            write!(out, "{}", crate::ast::TempId(results.0 + i));
        }

        if result_count.get() > 1 {
            out.write_str(")");
        }

        out.write_str(" = ");
    }
}

pub(crate) fn print_statements(
    out: &mut dyn crate::write::Write,
    context: &print::Context,
    function: crate::ast::FuncId,
    indentation: Indentation,
    mut indent_level: u32,
    statements: &[crate::ast::Statement],
) {
    use crate::ast::Statement;

    for (n, stmt) in statements.iter().copied().enumerate() {
        let is_last = n == statements.len() - 1;

        // Special handling of indentations for `return`s.
        if !matches!(
            stmt,
            Statement::BlockEnd { .. }
                | Statement::Else { .. }
                | Statement::BlockEndUnreachable { .. }
                | Statement::Branch {
                    target: crate::ast::BranchTarget::Return,
                    ..
                }
        ) {
            print_indentation(out, indentation, indent_level);
        }

        match stmt {
            Statement::Expr(expr) => {
                use crate::ast::Expr;

                debug_assert!(!is_last, "expected a terminator statement");

                match context.arena.get(expr) {
                    Expr::Literal(literal) => {
                        out.write_str("// ");
                        literal.print(out);
                    }
                    // No side effects or sub-expressions to evauluate.
                    Expr::GetGlobal(_)
                    | Expr::GetLocal(_)
                    | Expr::LoopInput(_)
                    | Expr::Temporary(_)
                    | Expr::MemorySize(_)
                    | Expr::TableSize(_) => (),
                    expr => {
                        const DISCARD: &str = "let _ = ";

                        match expr {
                            Expr::Call { callee, .. } => {
                                let has_error = context
                                    .wasm
                                    .function_attributes
                                    .unwind_kind(callee)
                                    .can_unwind();

                                let returns_values =
                                    !context.wasm.function_signature(callee).results().is_empty();

                                if has_error || returns_values {
                                    out.write_str(DISCARD);
                                }
                            }
                            Expr::UnaryOperator { .. }
                            | Expr::BinaryOperator { .. }
                            | Expr::MemoryLoad { .. }
                            | Expr::TableGet { .. } => out.write_str(DISCARD),
                            _ => (),
                        }

                        expr.print(out, false, context, Some(function));
                        out.write_str(";");
                    }
                }
            }
            Statement::Call {
                callee,
                arguments,
                results,
                offset,
            } => {
                print_call_statement_results(out, results);
                print::print_call_expr(out, callee, arguments, context, Some(function), offset);
                out.write_str(";");
            }
            Statement::CallIndirect {
                type_idx,
                table,
                callee,
                arguments,
                results,
                offset,
            } => {
                print_call_statement_results(out, results);
                out.write_str(paths::RT_FUNC_REF);
                write!(out, "::call_indirect_{}::<{}", arguments.len(), table.0);

                let signature = context.wasm.types
                    [context.wasm.types.core_type_at(type_idx).unwrap_sub()]
                .unwrap_func();
                for param in signature.params().iter().copied() {
                    write!(out, ", {}", crate::ast::ValType::from(param));
                }

                out.write_str(", ");

                if signature.results().len() != 1 {
                    out.write_str("(");
                }

                for (i, result) in signature.results().iter().copied().enumerate() {
                    if i > 0 {
                        out.write_str(", ");
                    }

                    write!(out, "{}", crate::ast::ValType::from(result));
                }

                if signature.results().len() != 1 {
                    out.write_str(")");
                }

                write!(out, ", _, _>(");
                print::print_table(out, table, context.wasm);
                out.write_str(", ");
                callee.print(out, false, context, Some(function));

                for arg in context.arena.get_list(arguments) {
                    out.write_str(", ");
                    arg.print(out, false, context, Some(function));
                }

                out.write_str(", ");
                print::print_frame(out, Some(function), offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Return,
                values: results,
                condition,
            } => {
                let can_unwind = context
                    .wasm
                    .function_attributes
                    .unwind_kind(function)
                    .can_unwind();

                let has_return =
                    condition.is_some() || can_unwind || !results.is_empty() || !is_last;

                if has_return {
                    print_indentation(out, indentation, indent_level);
                }

                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                if !is_last {
                    out.write_str("return");

                    if !results.is_empty() || can_unwind {
                        out.write_str(" ");
                    }
                }

                if can_unwind {
                    out.write_str("Ok(");
                }

                results.print(
                    out,
                    results.len() > 1 || (results.is_empty() && can_unwind),
                    context,
                    Some(function),
                );

                if can_unwind {
                    out.write_str(")");
                }

                if condition.is_some() {
                    out.write_str("; }");
                } else if !is_last {
                    out.write_str(";");
                }

                if has_return {
                    out.write_str("\n");
                }
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Block(block),
                values,
                condition,
            } => {
                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                write!(out, "break {block}");

                if !values.is_empty() {
                    out.write_str(" ");

                    values.print(out, values.len() > 1, context, Some(function));
                }

                out.write_str(";");

                if condition.is_some() {
                    out.write_str("}");
                }
            }
            Statement::Branch {
                target: crate::ast::BranchTarget::Loop(r#loop),
                values,
                condition,
            } => {
                if let Some(condition) = condition {
                    out.write_str("if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" { ");
                }

                print_loop_inputs(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level,
                    r#loop,
                    values,
                );

                write!(out, "continue {};", r#loop);

                if condition.is_some() {
                    out.write_str("}");
                }
            }
            Statement::BranchTable {
                values,
                targets,
                default_target,
                comparand,
            } => {
                out.write_str("match ");
                comparand.print(out, false, context, Some(function));
                out.write_str(" {\n");

                for (i, target) in context
                    .arena
                    .get_branch_targets(targets)
                    .iter()
                    .copied()
                    .enumerate()
                {
                    print_indentation(out, indentation, indent_level + 1);
                    write!(out, "{i} => ");
                    print_branch_table_case(
                        out,
                        context,
                        function,
                        indentation,
                        indent_level + 1,
                        is_last,
                        BranchTableCase { target, values },
                    );
                }

                print_indentation(out, indentation, indent_level + 1);
                out.write_str("_ => ");
                print_branch_table_case(
                    out,
                    context,
                    function,
                    indentation,
                    indent_level + 1,
                    is_last,
                    BranchTableCase {
                        target: default_target,
                        values,
                    },
                );

                print_indentation(out, indentation, indent_level);
                out.write_str("}");
            }
            Statement::DefineLocal(local, ty) => {
                use crate::ast::ValType;

                write!(out, "let mut {local} = ");
                match ty {
                    ValType::I32 => out.write_str("0i32"),
                    ValType::I64 => out.write_str("0i64"),
                    ValType::F32 => out.write_str("0f32"),
                    ValType::F64 => out.write_str("0f64"),
                    ValType::Ref(ref_ty) => print::print_null_ref(out, ref_ty),
                }

                out.write_str(";");
            }
            Statement::Temporary { temporary, value } => {
                write!(out, "let {temporary} = ");
                value.print(out, false, context, Some(function));
                out.write_str(";");
            }
            Statement::SetLocal { local, value } => {
                write!(out, "{local} = ");
                value.print(out, false, context, Some(function));
                out.write_str(";");
            }
            Statement::SetGlobal { global, value } => {
                out.write_str("embedder::rt::global::Global::set(");

                if let Some(import) = context.wasm.global_import(global) {
                    todo!("set global import {import:?}");
                } else {
                    write!(out, "&self.{global}")
                }

                out.write_str(", ");
                value.print(out, false, context, Some(function));
                out.write_str(");")
            }
            Statement::BlockStart {
                id,
                results,
                kind,
                r#type,
            } => {
                debug_assert!(!is_last);

                if let crate::ast::BlockKind::Loop { inputs } = kind {
                    for (i, expr) in context.arena.get_list(inputs).iter().enumerate() {
                        write!(
                            out,
                            "let mut {} = ",
                            crate::ast::LoopInput {
                                r#loop: id,
                                number: i as u32
                            }
                        );

                        expr.print(out, false, context, Some(function));
                        writeln!(out, ";");
                        print_indentation(out, indentation, indent_level);
                    }
                }

                if let Some(results) = results {
                    let result_types = match r#type {
                        wasmparser::BlockType::Empty => unreachable!(),
                        wasmparser::BlockType::Type(ref ty) => std::slice::from_ref(ty),
                        wasmparser::BlockType::FuncType(func_ty) => context.wasm.types
                            [context.wasm.types.core_type_at(func_ty).unwrap_sub()]
                        .unwrap_func()
                        .results(),
                    };

                    out.write_str("let ");

                    let has_many = results.count.get() > 1;
                    if has_many {
                        out.write_str("(");
                    }

                    for i in 0..results.count.get() {
                        if i > 0 {
                            out.write_str(", ");
                        }

                        write!(out, "{}", crate::ast::TempId(results.start.0 + i));
                    }

                    if has_many {
                        out.write_str(")");
                    }

                    // Write type annotations

                    out.write_str(": ");

                    if has_many {
                        out.write_str("(");
                    }

                    debug_assert_eq!(results.count.get(), result_types.len() as u32);
                    for (i, result_ty) in result_types.iter().copied().enumerate() {
                        if i > 0 {
                            out.write_str(", ");
                        }

                        write!(out, "{}", crate::ast::ValType::from(result_ty));
                    }

                    if has_many {
                        out.write_str(")");
                    }

                    out.write_str(" = ");
                }

                write!(out, "{id}: ");

                if matches!(kind, crate::ast::BlockKind::Loop { .. }) {
                    out.write_str("loop ");
                }

                out.write_str("{");

                if let crate::ast::BlockKind::If { condition } = kind {
                    out.write_str(" if ");
                    condition.print_bool(out, context, Some(function));
                    out.write_str(" {");
                }

                indent_level += 1;
            }
            Statement::Else { previous_results } => {
                debug_assert!(!is_last);

                if !previous_results.is_empty() {
                    print_indentation(out, indentation, indent_level);
                    previous_results.print(
                        out,
                        previous_results.len() > 1,
                        context,
                        Some(function),
                    );
                    out.write_str("\n");
                }

                indent_level -= 1;

                print_indentation(out, indentation, indent_level);
                out.write_str("} else {");

                indent_level += 1;
            }
            Statement::BlockEnd { id, kind, results } => {
                debug_assert!(!is_last);

                let is_loop = matches!(kind, crate::ast::BlockKind::Loop { .. });
                if is_loop || !results.is_empty() {
                    print_indentation(out, indentation, indent_level);
                }

                if is_loop {
                    write!(out, "break {id}");

                    if !results.is_empty() {
                        out.write_str(" ");
                    };
                }

                if !results.is_empty() {
                    results.print(out, results.len() > 1, context, Some(function));

                    if is_loop {
                        out.write_str(";");
                    }
                }

                if is_loop || !results.is_empty() {
                    out.write_str("\n");
                }

                indent_level -= 1;

                print_indentation(out, indentation, indent_level);

                if let crate::ast::BlockKind::If { condition: () } = kind {
                    out.write_str("} ");
                }

                out.write_str("}");

                if !results.is_empty() {
                    out.write_str(";");
                }

                write!(out, " // {id}");
            }
            Statement::BlockEndUnreachable {
                id,
                kind,
                has_results,
            } => {
                indent_level -= 1;

                print_indentation(out, indentation, indent_level);

                if let crate::ast::BlockKind::If { condition: () } = kind {
                    out.write_str("} ");
                }

                out.write_str("}");

                if has_results {
                    out.write_str(";");
                }

                write!(out, " // {id}");
            }
            Statement::TableSet {
                table,
                index,
                value,
                instruction_offset,
            } => {
                out.write_str(paths::RT_TABLE);
                write!(
                    out,
                    "::set::<{}, embedder::{table}, embedder::Trap>(",
                    table.0
                );
                print::print_table(out, table, context.wasm);
                out.write_str(", ");
                index.print(out, false, context, Some(function));
                out.write_str(", ");
                value.print(out, false, context, Some(function));
                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::TableFill {
                table,
                index,
                value,
                length,
                instruction_offset,
            } => {
                out.write_str(paths::RT_TABLE);
                write!(out, "::fill::<{}, _, _>(", table.0);
                print::print_table(out, table, context.wasm);
                out.write_str(", ");
                index.print(out, false, context, Some(function));
                out.write_str(", ");
                value.print(out, false, context, Some(function));
                out.write_str(", ");
                length.print(out, false, context, Some(function));
                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::MemoryStore {
                memory,
                kind,
                address,
                value,
                offset,
                instruction_offset,
            } => {
                use crate::ast::StoreKind;

                out.write_str(paths::RT_MEM);
                out.write_str("::");
                out.write_str(match kind {
                    StoreKind::I8 => "i8",
                    StoreKind::I16 => "i16",
                    StoreKind::I32 | StoreKind::AsI32 | StoreKind::F32 => "i32",
                    StoreKind::I64 | StoreKind::F64 => "i64",
                });
                write!(out, "_store::<{}, ", memory.0);
                let memory64 = context.wasm.types.memory_at(memory.0).memory64;
                out.write_str(if memory64 { "u64" } else { "u32" });
                write!(out, ", embedder::{memory}, embedder::Trap>(");
                print::print_memory(out, memory, context.wasm);
                write!(out, ", {offset}, ",);
                address.print(out, false, context, Some(function));
                out.write_str(", ");

                match kind {
                    StoreKind::F32 => out.write_str("f32::to_bits("),
                    StoreKind::F64 => out.write_str("f64::to_bits("),
                    _ => (),
                }

                value.print(
                    out,
                    matches!(kind, StoreKind::I8 | StoreKind::I16 | StoreKind::AsI32),
                    context,
                    Some(function),
                );

                match kind {
                    StoreKind::I8 => out.write_str(" as i8"),
                    StoreKind::I16 => out.write_str(" as i16"),
                    StoreKind::AsI32 => out.write_str(" as i32"),
                    StoreKind::F32 => out.write_str(") as i32"),
                    StoreKind::F64 => out.write_str(") as i64"),
                    _ => (),
                }

                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::MemoryFill {
                memory,
                address,
                byte,
                length,
                instruction_offset,
            } => {
                out.write_str(paths::RT_MEM);
                write!(out, "::fill::<{}, _, _, _>(", memory.0);
                print::print_memory(out, memory, context.wasm);
                out.write_str(", ");
                address.print(out, false, context, Some(function));
                out.write_str(", ");
                byte.print(out, false, context, Some(function));
                out.write_str(", ");
                length.print(out, false, context, Some(function));
                out.write_str(", ");
                print_frame(out, Some(function), instruction_offset, context.debug_info);
                out.write_str(")?;");
            }
            Statement::Unreachable { offset } => {
                out.write_str("return ::core::result::Result::Err(");

                write!(
                    out,
                    "{}::trap(embedder::rt::trap::UnreachableError",
                    paths::RT_TRAP
                );

                out.write_str(", ");
                print_frame(out, Some(function), offset, context.debug_info);
                out.write_str("));");
            }
        }

        // Special handling for newlines for `return`s.
        if !matches!(
            stmt,
            Statement::Branch {
                target: crate::ast::BranchTarget::Return,
                ..
            }
        ) {
            out.write_str("\n");
        }
    }
}
