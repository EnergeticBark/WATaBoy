import { testROMPath, wasmModulePath } from "./common.js";
import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	"rom_1Mb.gb",
    "rom_2Mb.gb",
	"rom_4Mb.gb",
	"rom_8Mb.gb",
	"rom_16Mb.gb",
	"rom_32Mb.gb",
	"rom_64Mb.gb",
	"rom_512kb.gb",
];

test.each(roms)("%p", async (romName) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file(wasmModulePath).arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file(testROMPath + "mooneye/mbc5/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runMooneyeTest();
	expect(passed).toBeTruthy();
});
