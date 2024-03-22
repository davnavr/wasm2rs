use std::fmt::Write;

const IMPORTS_OBJECT: &str = "self._embedder.imports()";
const LIMITS_ENUM: &str = "embedder::rt::trap::LimitsCheck";

pub(in crate::translation) fn write(
    buffer_pool: &crate::buffer::Pool,
    section: wasmparser::ImportSectionReader,
    types: &wasmparser::types::Types,
) -> wasmparser::Result<crate::translation::GeneratedLines> {
    let mut impl_out = crate::buffer::Writer::new(buffer_pool);
    let mut init_out = crate::buffer::Writer::new(buffer_pool);

    impl_out.write_str("    // Imports\n");

    let mut name_map = indexmap::IndexMap::<_, indexmap::IndexSet<_>>::new();

    let mut function_index = 0u32;
    let mut memory_index = 0u32;
    let mut global_index = 0u32;
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
                    " {{ {IMPORTS_OBJECT}.{import_module}().{import_name}("
                );

                let param_count = u32::try_from(signature.params().len()).unwrap();
                for i in 0..param_count {
                    if i > 0 {
                        impl_out.write_str(", ");
                    }

                    let _ = write!(impl_out, "{}", crate::translation::display::LocalId(i));
                }

                impl_out.write_str(")? }\n");

                function_index += 1;
            }
            ImportKind::Memory(mem_type) => {
                init_out.write_str("      {\n        ");

                // Emit code to check imported memory against limits
                let _ = writeln!(
                    init_out,
                    "let import = embedder.imports().{import_module}().{import_name}();"
                );

                init_out.write_str("        let min = import.size();\n");

                let _ = writeln!(init_out, "        if min < {} {{", mem_type.initial,);

                let _ = writeln!(
                    init_out,
                    "          return Err({}::trap(&embedder, {}::MemoryLimitsCheck {{",
                    crate::translation::function::TRAP_TRAIT,
                    crate::translation::function::TRAP_CODE,
                );

                let _ = writeln!(init_out, "            memory: {memory_index},",);

                let _ = writeln!(
                    init_out,
                    "            limits: {LIMITS_ENUM}::Minimum {{ expected: {}, actual: min }},",
                    mem_type.initial
                );

                init_out.write_str("          }));\n        }\n");

                let maximum = mem_type.maximum.unwrap_or(u32::MAX.into());
                init_out.write_str("        let max = import.limit();\n");
                let _ = writeln!(init_out, "        if max < {maximum} {{",);

                let _ = writeln!(
                    init_out,
                    "          return Err({}::trap(&embedder, {}::MemoryLimitsCheck {{",
                    crate::translation::function::TRAP_TRAIT,
                    crate::translation::function::TRAP_CODE,
                );

                let _ = writeln!(init_out, "            memory: {memory_index},",);

                let _ = writeln!(
                    init_out,
                    "            limits: {LIMITS_ENUM}::Maximum {{ expected: {maximum}, actual: max }},",
                );

                init_out.write_str("          }));\n        }\n      }\n");

                // Write the method used to access the memory
                let _ = writeln!(
                    impl_out,
                    "{}(&self) -> &embedder::Memory{memory_index} {{",
                    crate::translation::display::MemId(memory_index)
                );

                let _ = writeln!(
                    impl_out,
                    "      {IMPORTS_OBJECT}.{import_module}().{import_name}()"
                );

                impl_out.write_str("    }\n");

                memory_index += 1;
            }
            ImportKind::Global(global_type) => {
                let _ = write!(
                    impl_out,
                    "{}(&self) -> &",
                    crate::translation::display::GlobalId(global_index)
                );

                if global_type.mutable {
                    impl_out.write_str("embedder::rt::global::Global<");
                }

                let _ = write!(
                    impl_out,
                    "{}",
                    crate::translation::display::ValType(global_type.content_type)
                );

                if global_type.mutable {
                    impl_out.write_str(">");
                }

                let _ = writeln!(
                    impl_out,
                    " {{ {IMPORTS_OBJECT}.{import_module}().{import_name}() }}"
                );

                global_index += 1;
            }
            bad => todo!("importing {bad:?} is not yet supported"),
        }
    }

    impl_out.write_str("\n");

    Ok(crate::translation::GeneratedLines {
        impls: impl_out.finish(),
        inits: init_out.finish(),
        ..Default::default()
    })
}
