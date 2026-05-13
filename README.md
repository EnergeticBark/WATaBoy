# WATaBoy
A Game Boy emulator with an SM83 to Wasm JIT compiler.

## Building
### Wasm
Run the following command:
```shell
cargo build --package jit -Zbuild-std=core,std,alloc,proc_macro,panic_abort --target=wasm32-unknown-unknown --release
```
Then, optionally (but preferably) pass the .wasm file through `wasm-opt` from [Binaryen](https://github.com/WebAssembly/binaryen):
```shell
wasm-opt target/wasm32-unknown-unknown/release/jit.wasm -O -o target/wasm32-unknown-unknown/release/jit.wasm
```

## Testing
### Wasm
[Bun](https://bun.com/) is used automate testing the Wasm JIT headlessly without needing to run a full web browser.

To run the test suite, build the Wasm module normally, and then run `bun test` in the project's root directory.
