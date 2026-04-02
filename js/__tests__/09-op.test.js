import { Runtime } from "../runtime.js";

const runtime = new Runtime();
const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/sm83_jit.wasm").arrayBuffer();
await runtime.init(wasmSource);
const rom = await Bun.file("./js/__tests__/roms/09-op r,r.gb").bytes();
runtime.loadRom(rom);

test("09-op r,r.gb", () => {
	let passed = runtime.runBlarggTest(0xCE67, 3);
    expect(passed).toBeTruthy();
});
