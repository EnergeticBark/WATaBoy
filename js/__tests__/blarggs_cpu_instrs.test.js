import { Runtime } from "../runtime.js";

// Parameters for the parameterised test.
// The first param is the rom name, the second param is the line in the tile map where "Passed" will appear. 
const roms = [
	["01-special.gb", 3],
	["02-interrupts.gb", 3],
	["03-op sp,hl.gb", 3],
	["04-op r,imm.gb", 3],
	["05-op rp.gb", 3],
	["06-ld r,r.gb", 3],
	["07-jr,jp,call,ret,rst.gb", 4],
	["08-misc instrs.gb", 3],
	["09-op r,r.gb", 3],
	["10-bit ops.gb", 3],
	["11-op a,(hl).gb", 3],
];

test.each(roms)("%p", async (romName, passed_line) => {
	const runtime = new Runtime();
	const wasmSource = await Bun.file("./target/wasm32-unknown-unknown/release/sm83_jit.wasm").arrayBuffer();
	await runtime.init(wasmSource);
	const rom = await Bun.file("./js/__tests__/roms/" + romName).bytes();
	runtime.loadRom(rom);
	
	let passed = runtime.runBlarggTest(passed_line);
    expect(passed).toBeTruthy();
});
