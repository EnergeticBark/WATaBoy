use wasm_encoder::*;

pub(crate) fn empty_jit_block_module() -> Module {
    let mut module = Module::new();

    // Encode the type section.
    let mut types = TypeSection::new();
    // Parameters: A, F, B, C, D, E, H, and L registers.
    let params = vec![ValType::I32; 8];
    // Return those same registers, but modified.
    let results = vec![ValType::I32; 8];
    types.ty().function(params, results);

    // Parameters: 16-bit index into memory, 8-bit value to write, current system clock.
    let params = vec![ValType::I32; 3];
    let results = vec![];
    types.ty().function(params, results);
    module.section(&types);

    let mut imports = ImportSection::new();
    // The write_byte function uses index 1 in the types section.
    imports.import("env", "write_byte", EntityType::Function(1));

    module.section(&imports);

    // Encode the function section.
    let mut functions = FunctionSection::new();
    let type_index = 0;
    functions.function(type_index);
    module.section(&functions);

    // Encode the export section
    let mut exports = ExportSection::new();
    exports.export("execute_block", ExportKind::Func, 1);
    module.section(&exports);

    module
}

pub(crate) fn empty_jit_block_function() -> Function {
    // Provide two locals, 8 and 9, to be used as "scratch registers".
    let locals = vec![(2, ValType::I32)];
    Function::new(locals)
}
