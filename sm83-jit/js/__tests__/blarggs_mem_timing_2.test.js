import { testROMPath } from "./common.js";
import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	["01-read_timing.gb", 3],
	["02-write_timing.gb", 3],
	["03-modify_timing.gb", 3],
];

test.each(roms)("%p", async (romName, passed_line) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/sm83_jit.wasm").arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file(testROMPath + "blarggs/mem_timing_2/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runBlarggTest(passed_line);
	expect(passed).toBeTruthy();
});
