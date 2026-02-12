fn main() {
    // Imports the Wasm function table from the environment.
    // See: https://lld.llvm.org/WebAssembly.html
    println!("cargo:rustc-link-arg=--import-table");
}
