use std::fmt::Write;

fn write_result_types(out: &mut crate::buffer::Writer, types: &[wasmparser::ValType]) {
    out.write_str("&[");

    for (i, ty) in types.iter().enumerate() {
        use wasmparser::ValType;

        if i > 0 {
            out.write_str(", ");
        }

        out.write_str("embedder::rt::stack::trace::WasmValType::");
        out.write_str(match ty {
            ValType::I32 => "I32",
            ValType::I64 => "I64",
            ValType::F32 => "F32",
            ValType::F64 => "F64",
            ValType::V128 => "V128",
            &ValType::FUNCREF => "FuncRef",
            &ValType::EXTERNREF => "ExternRef",
            ValType::Ref(unsupported) => {
                unimplemented!("unknown value type in signature {unsupported:?}")
            }
        });
    }

    out.write_str("]");
}

fn write_function_signature(
    out: &mut crate::buffer::Writer,
    func_idx: u32,
    signature: &wasmparser::FuncType,
) {
    let _ = write!(
        out,
        "    const {}: embedder::rt::stack::trace::WasmSymbolSignature = \
        embedder::rt::stack::trace::WasmSymbolSignature {{ parameters: ",
        crate::translation::display::FuncSignature(func_idx),
    );

    write_result_types(out, signature.params());
    out.write_str(", results: ");
    write_result_types(out, signature.results());
    out.write_str(" };\n");
}

pub fn write(
    buffer_pool: &crate::buffer::Pool,
    types: &wasmparser::types::Types,
) -> crate::translation::GeneratedLines {
    let mut impl_out = crate::buffer::Writer::new(buffer_pool);

    for func_idx in 0u32..types.core_function_count() {
        write_function_signature(
            &mut impl_out,
            func_idx,
            crate::translation::function::get_function_type(
                &types[types.core_function_at(func_idx)],
            ),
        )
    }

    impl_out.write_str("\n");

    crate::translation::GeneratedLines {
        impls: impl_out.finish(),
        ..Default::default()
    }
}
