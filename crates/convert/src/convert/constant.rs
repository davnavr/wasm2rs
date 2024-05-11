use anyhow::Context;

/// Generates an AST from a [constant WebAssembly expression].
///
/// [constant WebAssembly expression]: https://webassembly.github.io/spec/core/valid/instructions.html#constant-expressions
pub(in crate::convert) fn create_ast(
    expr: &wasmparser::ConstExpr,
    arena: &mut crate::ast::Arena,
) -> crate::Result<crate::ast::ExprId> {
    //let mut operand_stack = ;
    let mut value = None;
    let mut reader = expr.get_operators_reader();
    loop {
        use crate::ast::Literal;
        use wasmparser::Operator;

        let (op, op_offset) = reader.read_with_offset()?;
        match op {
            Operator::I32Const { value: n } => {
                value = Some(arena.allocate(Literal::I32(n))?);
            }
            Operator::I64Const { value: n } => {
                value = Some(arena.allocate(Literal::I64(n))?);
            }
            Operator::F32Const { value: z } => {
                value = Some(arena.allocate(Literal::F32(z.bits()))?);
            }
            Operator::F64Const { value: z } => {
                value = Some(arena.allocate(Literal::F64(z.bits()))?);
            }
            Operator::RefNull { hty: _ } | Operator::RefFunc { function_index: _ } => {
                anyhow::bail!("references in constant expressions are currently not supported")
            }
            Operator::GlobalGet { global_index: _ } => {
                anyhow::bail!("global.get within a constant expression is not yet supported")
            }
            Operator::End => {
                if let Some(value) = value {
                    reader.ensure_end().with_context(|| {
                        format!("expected end of constant expression @ {op_offset:#X}")
                    })?;
                    return Ok(value);
                } else {
                    anyhow::bail!("invalid constant expression @ {op_offset:#X}, expected result on top of the stack");
                }
            }
            _ => anyhow::bail!("invalid constant instruction {op:?} @ {op_offset:#X}"), // `SUPPORTED_FEATURES` currently excludes `extended_const`
        }
    }
}
