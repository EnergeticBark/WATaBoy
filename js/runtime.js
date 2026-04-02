export const LCD_WIDTH = 160;
export const LCD_HEIGHT = 144;

const utf8decoder = new TextDecoder();

export const Runtime = class {
	// TODO: Determine this value at runtime instead of hardcoding it.
	lowestSafeFuncIdx = 5000;
	__indirect_function_table;
	instance;
	jitRuntimePtr;
	
	constructor() {
		this.__indirect_function_table = new WebAssembly.Table({ initial: 100000, element: "anyfunc" });
	}
	
	async init(source) {
		const importObj = {env: {
			console_log_glue: this.console_log_glue,
			instantiate_and_link_module: this.instantiate_and_link_module,
			__indirect_function_table: this.__indirect_function_table
		}};
		
		const {instance} = await WebAssembly.instantiate(source, importObj);
		this.instance = instance;
		
		this.jitRuntimePtr = instance.exports.make_runtime();
	}
	
	console_log_glue = (stringPtr, stringLen) => {
		const messageBytes = new Uint8Array(this.instance.exports.memory.buffer, stringPtr, stringLen);
		const message = utf8decoder.decode(messageBytes);
		console.log(message);
	}
	
	instantiate_and_link_module = (bufferPtr, bufferLen) => {
		console.log("Instantiate and link called...");
		
		const bytecode = new Uint8Array(this.instance.exports.memory.buffer, bufferPtr, bufferLen);
		
		const anotherMod = new WebAssembly.Module(bytecode);
		const anotherInstance = new WebAssembly.Instance(anotherMod, {});
		
		// This used to call the grow method, but that's busted in WebKit.
		// See: https://bugs.webkit.org/show_bug.cgi?id=290681
		this.__indirect_function_table.set(this.lowestSafeFuncIdx, anotherInstance.exports.execute_block)
		
		const prevIdx = this.lowestSafeFuncIdx;
		this.lowestSafeFuncIdx += 1;
		return prevIdx;
	}
	
	// Load the ROM file.
	loadRom = (rom) => {
		const romBufferPtr = this.instance.exports.realloc_rom_buffer(this.jitRuntimePtr, rom.length);
		const romBuffer = new Uint8Array(this.instance.exports.memory.buffer, romBufferPtr, rom.length);
		romBuffer.set(rom);
		this.instance.exports.load_rom_from_buffer(this.jitRuntimePtr);
	}
	
	updateLcd = (lcdImage) => {
		const lcdBufferPtr = this.instance.exports.get_lcd_buffer(this.jitRuntimePtr);
		const greyscalePixels = new Uint8Array(this.instance.exports.memory.buffer, lcdBufferPtr, LCD_WIDTH * LCD_HEIGHT);
		
		for (let greyscaleIndex = 0; greyscaleIndex < LCD_WIDTH * LCD_HEIGHT; greyscaleIndex += 1) {
			lcdImage.data[greyscaleIndex * 4] = greyscalePixels[greyscaleIndex];
			lcdImage.data[greyscaleIndex * 4 + 1] = greyscalePixels[greyscaleIndex];
			lcdImage.data[greyscaleIndex * 4 + 2] = greyscalePixels[greyscaleIndex];
		}
	}
	
	step_vblank = () => {
		this.instance.exports.step_vblank(this.jitRuntimePtr);
	}
	
	updateJoypad = (buttonsHeld) => {
		this.instance.exports.update_joypad(
			this.jitRuntime,
			buttonsHeld.start,
			buttonsHeld.select,
			buttonsHeld.b,
			buttonsHeld.a,
			buttonsHeld.down,
			buttonsHeld.up,
			buttonsHeld.left,
			buttonsHeld.right,
		);
	}
	
	runBlarggTest = passed_line => this.instance.exports.run_blargg_test(this.jitRuntimePtr, passed_line);
}