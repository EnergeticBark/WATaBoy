import { testROMPath, wasmModulePath } from "./common.js";
import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	"add_sp_e_timing.gb",
	// TODO: Implement boot ROM skipping so I can pass this without distributing the original mgb boot ROM.
	//"boot_div-dmgABCmgb.gb",
	//"boot_hwio-dmgABCmgb.gb",
	"boot_regs-mgb.gb",
	"call_cc_timing.gb",
	"call_cc_timing2.gb",
	"call_timing.gb",
	"call_timing2.gb",
	"di_timing-GS.gb",
	"div_timing.gb",
	"intr_timing.gb",
	"oam_dma_restart.gb",
	"oam_dma_start.gb",
	"oam_dma_timing.gb"
];

test.each(roms)("%p", async (romName) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file(wasmModulePath).arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file(testROMPath + "mooneye/" + romName).bytes();
	runtime.loadRom(rom);

	let passed = runtime.runMooneyeTest();
	expect(passed).toBeTruthy();
});
