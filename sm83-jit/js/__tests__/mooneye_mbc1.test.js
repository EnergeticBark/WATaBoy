import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	"bits_bank1.gb",
	"bits_bank2.gb",
	"bits_mode.gb",
	"bits_ramg.gb",
	"ram_64kb.gb"
];

test.each(roms)("%p", async (romName) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/sm83_jit.wasm").arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file("./js/__tests__/roms/mooneye/mbc1/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runMooneyeTest();
	expect(passed).toBeTruthy();
});
