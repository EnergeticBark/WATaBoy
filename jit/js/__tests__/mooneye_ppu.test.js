import { testROMPath } from "./common.js";
import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	"hblank_ly_scx_timing-GS.gb",
	"intr_1_2_timing-GS.gb",
	"intr_2_0_timing.gb",
	"intr_2_mode0_timing_sprites.gb",
	"intr_2_mode0_timing.gb",
	"intr_2_mode3_timing.gb",
	"intr_2_oam_ok_timing.gb",
	"lcdon_timing-GS.gb",
	"lcdon_write_timing-GS.gb",
	"stat_irq_blocking.gb",
	"stat_lyc_onoff.gb",
	"vblank_stat_intr-GS.gb"
];

test.each(roms)("%p", async (romName) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/jit.wasm").arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file(testROMPath + "mooneye/ppu/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runMooneyeTest();
	expect(passed).toBeTruthy();
});
