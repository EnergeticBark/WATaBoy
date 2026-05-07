import { testROMPath, wasmModulePath } from "./common.js";
import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	["instr_timing.gb", 3],
];

test.each(roms)("%p", async (romName, passed_line) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file(wasmModulePath).arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file(testROMPath + "blarggs/instr_timing/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runBlarggTest(passed_line);
	expect(passed).toBeTruthy();
});
