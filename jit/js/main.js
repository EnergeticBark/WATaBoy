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

const jitBlockCount = document.querySelector("#jit-block-count");

const errorLogContainer = document.querySelector("#error-log");
const errorLog = document.querySelector("#error-log output");

const runtime = new Runtime();
// Override the default error callback to show errors on the page.
runtime.errorCallback = (message) => {
	console.error(message);
	errorLog.textContent += message;
	errorLogContainer.hidden = false;
}
runtime.linkModuleCallback = () => {
	console.log("Link new module called...");
	jitBlockCount.textContent = runtime.linkedModuleCount;
}

const wasmSource = await (await fetch("../target/wasm32-unknown-unknown/release/jit.wasm")).bytes();
await runtime.init(wasmSource);

let nextFrame;
const renderLoop = (timestamp) => {
	// Cap the animation framerate to 60fps.
	if (timestamp >= nextFrame) {
		nextFrame += 1000 / 60;
		
		frametimeCounter.start();
	
		for (let i = 0; i < speedMultiplier; i += 1) {
			runtime.step_vblank();
		}
		
		frametimeCounter.end();
		runtime.updateLcd(lcdImage);
		ctx.putImageData(lcdImage, 0, 0);
		runtime.updateJoypad(buttonsHeld);
    }
	
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
	nextFrame = document.timeline.currentTime;
	renderLoop(nextFrame);
});

const logUncompiledButton = document.querySelector("#log-uncompiled");
logUncompiledButton.addEventListener("click", () => {
	runtime.logUncompiled();
});
