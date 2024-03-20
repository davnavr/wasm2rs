use std::fmt::Write;

/// Generates a Rust expression from a constant WebAssembly expression.
pub(in crate::translation) fn write(
    out: &mut crate::buffer::Writer,
    expr: &wasmparser::ConstExpr,
) -> wasmparser::Result<()> {
    use wasmparser::Operator;

    out.write_str("{");

    let mut ops = expr.get_operators_reader();
    let mut stack_height = 0usize;
    loop {
        match ops.read()? {
            Operator::Nop => (),
            // Operator::GlobalGet { global_index } => {
            //     let _ = writeln!(out, "let s_{stack_height} = self.{}.;", crate::translation::display::GlobalId(global_index));
            //     stack_height += 1;
            // },
            Operator::I32Const { value } => {
                let _ = writeln!(out, "let s_{stack_height} = {value}i32;");
                stack_height += 1;
            }
            Operator::I64Const { value } => {
                let _ = writeln!(out, "let s_{stack_height} = {value}i64;");
                stack_height += 1;
            }
            Operator::I32Add => {
                let c_2 = stack_height - 1;
                let c_1 = stack_height - 2;
                let _ = writeln!(out, "let {c_1} = i32::wrapping_add({c_1}, {c_2});",);
                stack_height -= 1;
            }
            Operator::End => {
                let _ = write!(out, "{}\n}}", stack_height - 1);
                return ops.ensure_end();
            }
            bad => {
                todo!("validation did not detect invalid constant expression containing {bad:?}")
            }
        }
    }
}
