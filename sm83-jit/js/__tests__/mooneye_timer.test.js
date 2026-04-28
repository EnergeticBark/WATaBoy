import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	"div_write.gb",
	"rapid_toggle.gb",
	"tim00_div_trigger.gb",
	"tim00.gb",
	"tim01_div_trigger.gb",
	"tim01.gb",
	"tim10_div_trigger.gb",
	"tim10.gb",
	"tim11_div_trigger.gb",
	"tim11.gb",
	"tima_reload.gb",
	"tima_write_reloading.gb",
	"tma_write_reloading.gb"
];

test.each(roms)("%p", async (romName) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/sm83_jit.wasm").arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file("./js/__tests__/roms/mooneye/timer/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runMooneyeTest();
	expect(passed).toBeTruthy();
});
