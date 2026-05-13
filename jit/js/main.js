import { buttonsHeld } from "./keyboard.js";
import { frametimeCounter } from "./frametime.js"
import { Runtime, LCD_WIDTH, LCD_HEIGHT } from "./runtime.js"

const speedInput = document.querySelector("#speed");
let speedMultiplier = speedInput.value;
speedInput.addEventListener("input", () => {
	speedMultiplier = speedInput.value;
});

const lcdImage = new ImageData(LCD_WIDTH, LCD_HEIGHT);
// Just fill lcdImage with all white pixels at full opacity so we don't need to worry about setting alpha values later.
lcdImage.data.fill(255);

const lcdCanvas = document.querySelector("#lcd");
const ctx = lcdCanvas.getContext("2d", { alpha: false });

const runtime = new Runtime();
const wasmSource = await (await fetch("../target/wasm32-unknown-unknown/release/jit-opt.wasm")).bytes();
await runtime.init(wasmSource);

const renderLoop = () => {
	frametimeCounter.start();
	for (let i = 0; i < speedMultiplier; i += 1) {
		runtime.step_vblank();
	}
	frametimeCounter.end();
	runtime.updateLcd(lcdImage);
	ctx.putImageData(lcdImage, 0, 0);
	runtime.updateJoypad(buttonsHeld);

	requestAnimationFrame(renderLoop);
};

const romFieldset = document.querySelector("fieldset");
const romInput = document.querySelector("#rom");
const loadRomButton = document.querySelector("#load-rom");
loadRomButton.addEventListener("click", async () => {
	// Gotta refresh to load a new ROM.
	romFieldset.disabled = true;
	
	// Read bytes from the selected file.
	const file = romInput.files[0];
	const rom = await file.bytes();
	
	runtime.loadRom(rom);
	
	// Start the render loop.
	renderLoop();
});

const logUncompiledButton = document.querySelector("#log-uncompiled");
logUncompiledButton.addEventListener("click", () => {
	runtime.logUncompiled();
});
