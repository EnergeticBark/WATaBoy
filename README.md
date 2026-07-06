# WATaBoy
A high-accuracy Game Boy emulator with an SM83 to Wasm JIT compiler. [Try it out!](https://humphri.es/WATaBoy/)

<img width="2940" height="1846" alt="Screenshot of WATaBoy with Chrome DevTools showing a JIT compiled Wasm module's source" src="https://github.com/user-attachments/assets/ee6ffff5-a7f7-4bdd-9260-80a29946ae00">

## Features
WATaBoy's JIT dynamically recompiles Game Boy CPU instructions to Wasm, making it cross-platform (even on iOS).

JIT-ing to Wasm in a web browser consistently outperforms the interpreter running natively; read more on [the WATaBoy blog post](https://humphri.es/blog/WATaBoy/).

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
[Bun](https://bun.com/) is used to automate headless testing of the Wasm JIT without requiring a full web browser.

To run the test suite, build the Wasm module, then run `bun test` in the project's root directory.
