fn main() {
    // Exports the Wasm function table from the environment.
    // See: https://lld.llvm.org/WebAssembly.html#cmdoption-export-table
    println!("cargo:rustc-link-arg=--export-table");
    // Make the exported table growable so we can link our JIT compiled functions.
    // This flag is totally undocumented, but it works, and there's a test for it, so...
    // See: https://github.com/llvm/llvm-project/blob/04025adc8c6b9fd4542e1c658ed19381d4274ea0/lld/test/wasm/growable-table.test#L2
    println!("cargo:rustc-link-arg=--growable-table");
    println!("cargo:rustc-link-arg=--allow-undefined");
}
