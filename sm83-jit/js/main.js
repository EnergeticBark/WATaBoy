import { buttonsHeld } from "./keyboard.js";
import { frametimeCounter } from "./frametime.js"
import { Runtime, LCD_WIDTH, LCD_HEIGHT } from "./runtime.js"

const lcdImage = new ImageData(LCD_WIDTH, LCD_HEIGHT);
// Just fill lcdImage with all white pixels at full opacity so we don't need to worry about setting alpha values later.
lcdImage.data.fill(255);

const lcdCanvas = document.querySelector("#lcd");
const ctx = lcdCanvas.getContext("2d", { alpha: false });

const runtime = new Runtime();
const wasmSource = await (await fetch("../target/wasm32-unknown-unknown/release/sm83_jit.wasm")).bytes();
await runtime.init(wasmSource);
const rom = await (await fetch("./Pokemon - Blue Version (USA, Europe) (SGB Enhanced).sgb")).bytes();
runtime.loadRom(rom);

const renderLoop = () => {
	frametimeCounter.start();
	for (let i = 0; i < 500; i += 1) {
		runtime.step_vblank();
	}
	frametimeCounter.end();
	runtime.updateLcd(lcdImage);
	ctx.putImageData(lcdImage, 0, 0);
	runtime.updateJoypad(buttonsHeld);

	requestAnimationFrame(renderLoop);
};
renderLoop();

const logUncompiledButton = document.querySelector("#log-uncompiled");
logUncompiledButton.addEventListener("click", () => {
	runtime.logUncompiled();
});
