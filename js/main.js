import { buttonsHeld } from "./keyboard.js";

const utf8decoder = new TextDecoder();
export const console_log_glue = (stringPtr, stringLen) => {
	const messageBytes = new Uint8Array(instance.exports.memory.buffer, stringPtr, stringLen);
	const message = utf8decoder.decode(messageBytes);
	console.log(message);
}

const LCD_WIDTH = 160;
const LCD_HEIGHT = 144;

const lcdImage = new ImageData(LCD_WIDTH, LCD_HEIGHT);
// Just fill lcdImage with all white pixels at full opacity so we don't need to worry about setting alpha values later.
lcdImage.data.fill(255);

const lcdCanvas = document.querySelector("#lcd");
const ctx = lcdCanvas.getContext("2d", { alpha: false });

// TODO: Determine this value at runtime instead of hardcoding it.
let lowestSafeFuncIdx = 5000;
const instantiate_and_link_module = (bufferPtr, bufferLen) => {
	console.log("Instantiate and link called...");
				
	const bytecode = new Uint8Array(instance.exports.memory.buffer, bufferPtr, bufferLen);
	
	const anotherMod = new WebAssembly.Module(bytecode);
	const anotherInstance = new WebAssembly.Instance(anotherMod, {});
	
	// This used to call the grow method, but that's busted in WebKit.
	// See: https://bugs.webkit.org/show_bug.cgi?id=290681
	__indirect_function_table.set(lowestSafeFuncIdx, anotherInstance.exports.execute_block)
	
	const prevIdx = lowestSafeFuncIdx;
	lowestSafeFuncIdx += 1;
	return prevIdx;
}

const __indirect_function_table = new WebAssembly.Table({ initial: 10000, element: "anyfunc" });

const importObj = {env: {console_log_glue, instantiate_and_link_module, __indirect_function_table}};

const source = fetch("./target/wasm32-unknown-unknown/release/sm83_jit.wasm");
const {instance} = await WebAssembly.instantiateStreaming(source, importObj);

const jitRuntime = instance.exports.make_runtime();

const update_lcd = () => {
	const lcdBufferPtr = instance.exports.get_lcd_buffer(jitRuntime);
	const greyscalePixels = new Uint8Array(instance.exports.memory.buffer, lcdBufferPtr, LCD_WIDTH * LCD_HEIGHT);
	
	for (let greyscaleIndex = 0; greyscaleIndex < LCD_WIDTH * LCD_HEIGHT; greyscaleIndex += 1) {
		lcdImage.data[greyscaleIndex * 4] = greyscalePixels[greyscaleIndex];
		lcdImage.data[greyscaleIndex * 4 + 1] = greyscalePixels[greyscaleIndex];
		lcdImage.data[greyscaleIndex * 4 + 2] = greyscalePixels[greyscaleIndex];
	}
	
	ctx.putImageData(lcdImage, 0, 0)
}

setInterval(() => {
	instance.exports.step_vblank(jitRuntime);
	update_lcd();
	instance.exports.update_joypad(
		jitRuntime,
		buttonsHeld.start,
		buttonsHeld.select,
		buttonsHeld.b,
		buttonsHeld.a,
		buttonsHeld.down,
		buttonsHeld.up,
		buttonsHeld.left,
		buttonsHeld.right,
	);
}, 1);

console.log("done :)");