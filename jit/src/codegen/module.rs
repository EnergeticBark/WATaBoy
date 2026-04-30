use wasm_encoder::*;

pub const PROLOGE_LENGTH: usize = 9;

pub(crate) fn empty_jit_block_module() -> Module {
    let mut module = Module::new();

    // Encode the type section.
    let mut types = TypeSection::new();
    // Parameters: A, F, B, C, D, E, H, L, and SP registers.
    let params = vec![ValType::I32; PROLOGE_LENGTH];
    // Return those same registers, but modified.
    let results = vec![ValType::I32; PROLOGE_LENGTH];
    types.ty().function(params, results);

    // Type for the read_byte function.
    // Parameters: 16-bit index into memory, current system clock, runtime pointer.
    // Returns: 8-bit value at the specified index.
    let params = vec![ValType::I32; 3];
    let results = vec![ValType::I32];
    types.ty().function(params, results);

    // Type for the write_byte function.
    // Parameters: 8-bit value to write, 16-bit index into memory, current system clock, runtime pointer.
    let params = vec![ValType::I32; 4];
    let results = vec![];
    types.ty().function(params, results);

    // Type for the process_checkpoint function.
    // Parameter: 32-bit checkpoint index, runtime pointer.
    // Returns: Boolean, whether the current block should be aborted.
    let params = vec![ValType::I32; 2];
    let results = vec![ValType::I32];
    types.ty().function(params, results);
    module.section(&types);

    let mut imports = ImportSection::new();
    // The read_byte function uses index 1 in the types section.
    imports.import("env", "read_byte", EntityType::Function(1));
    // The write_byte function uses index 2 in the types section.
    imports.import("env", "write_byte", EntityType::Function(2));
    // The process_checkpoint function uses index 3 in the types section.
    imports.import("env", "process_checkpoint", EntityType::Function(3));

    module.section(&imports);

    // Encode the function section.
    let mut functions = FunctionSection::new();
    let type_index = 0;
    functions.function(type_index);
    module.section(&functions);

    // Encode the export section
    let mut exports = ExportSection::new();
    exports.export("execute_block", ExportKind::Func, 3);
    module.section(&exports);

    module
}

pub(crate) fn empty_jit_block_function() -> Function {
    // Provide two locals to be used as "scratch registers".
    let locals = vec![(2, ValType::I32)];
    Function::new(locals)
}
