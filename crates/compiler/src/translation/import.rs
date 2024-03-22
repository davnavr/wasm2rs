use std::fmt::Write;

pub(in crate::translation) fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::ImportSectionReader,
    types: &wasmparser::types::Types,
) -> wasmparser::Result<crate::translation::GeneratedLines> {
    let mut impl_out = crate::buffer::Writer::new(buffer_pool);

    impl_out.write_str("    // Imports\n");

    let mut name_map = indexmap::IndexMap::<_, indexmap::IndexSet<_>>::new();

    let mut function_index = 0u32;
    for result in section {
        use wasmparser::TypeRef as ImportKind;

        let import = result?;

        if !name_map
            .entry(import.module)
            .or_default()
            .insert(import.name)
        {
            todo!(
                "conflicting imports ({:?} from {:?}) are not yet supported",
                import.name,
                import.module
            );
        }

        impl_out.write_str("    fn ");

        let import_module = crate::rust::SafeIdent::from(import.module);
        let import_name = crate::rust::SafeIdent::from(import.name);
        match import.ty {
            ImportKind::Func(ty_index) => {
                let signature = types[types.core_type_at(ty_index).unwrap_sub()].unwrap_func();

                let _ = write!(
                    impl_out,
                    "{}",
                    crate::translation::display::FuncId(function_index)
                );

                crate::translation::function::write_definition_signature(&mut impl_out, signature);

                let _ = write!(
                    impl_out,
                    " {{ self._embedder.imports().{import_module}().{import_name}("
                );

                let param_count = u32::try_from(signature.params().len()).unwrap();
                for i in 0..param_count {
                    if i > 0 {
                        impl_out.write_str(", ");
                    }

                    let _ = write!(impl_out, "{}", crate::translation::display::LocalId(i));
                }

                impl_out.write_str(")? }}\n");

                function_index += 1;
            }
            bad => todo!("importing {bad:?} is not yet supported"),
        }
    }

    impl_out.write_str("\n");

    Ok(crate::translation::GeneratedLines {
        impls: impl_out.finish(),
        ..Default::default()
    })
}
