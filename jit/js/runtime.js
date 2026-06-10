export const LCD_WIDTH = 160;
export const LCD_HEIGHT = 144;

const utf8decoder = new TextDecoder();

export const Runtime = class {
	instance;
	jitRuntimePtr;
	
	async init(source) {
		const importObj = {env: {
			console_error_glue: this.#console_error_glue,
			console_log_glue: this.#console_log_glue,
			link_new_module_glue: this.link_new_module_glue,
		}};
		
		const {instance} = await WebAssembly.instantiate(source, importObj);
		this.instance = instance;
		this.jitRuntimePtr = instance.exports.make_runtime();
	}
	
	#decodeUTF8 = (stringPtr, stringLen) => {
		const messageBytes = new Uint8Array(this.instance.exports.memory.buffer, stringPtr, stringLen);
		return utf8decoder.decode(messageBytes);
	}
	
	errorCallback = console.error;
	logCallback = console.log;
	
	#console_error_glue = (stringPtr, stringLen) => {
		const message = this.#decodeUTF8(stringPtr, stringLen);
		this.errorCallback(message);
	};
	
	#console_log_glue = (stringPtr, stringLen) => {
		const message = this.#decodeUTF8(stringPtr, stringLen);
		this.logCallback(message);
	}
	
	link_new_module_glue = (bufferPtr, bufferLen) => {
		console.log("Link new module called...");
		
		// Read the Wasm bytecode from the JIT runtime's memory.
		const bytecode = new Uint8Array(this.instance.exports.memory.buffer, bufferPtr, bufferLen);
		
		// Compile and instantiate the bytecode into a new instance.
		const newModule = new WebAssembly.Module(bytecode);
		const importObj = {env: {
			runtime_mem: this.instance.exports.memory,
			process_checkpoint: this.instance.exports.process_checkpoint,
			read_byte: this.instance.exports.read_byte_mem,
			write_byte: this.instance.exports.write_byte_mem,
		}};
		const newInstance = new WebAssembly.Instance(newModule, importObj);
		
		// Add the instance's "execute_block" function to our JIT runtime's function table.
		// We should be able to get rid of `set` and just pass `execute_block` as the 2nd argument to `grow`, but that's busted in WebKit.
		// See: https://bugs.webkit.org/show_bug.cgi?id=290681
		this.instance.exports.__indirect_function_table.grow(1)
		const newFuncIdx = this.instance.exports.__indirect_function_table.length - 1;
		this.instance.exports.__indirect_function_table.set(newFuncIdx, newInstance.exports.execute_block)
		
		// Return the index of the function we've just linked in.
		return newFuncIdx;
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
			this.jitRuntimePtr,
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
	runMooneyeTest = () => this.instance.exports.run_mooneye_test(this.jitRuntimePtr);
	
	logUncompiled = () => {
		this.instance.exports.log_uncompiled(this.jitRuntimePtr);
	};
}