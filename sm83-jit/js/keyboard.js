export const buttonsHeld = {
	start: false,
	select: false,
	b: false,
	a: false,
	down: false,
	up: false,
	left: false,
	right: false,
};

document.addEventListener("keydown", (event) => {
	switch (event.key) {
		case "Enter":
			buttonsHeld.start = true;
			break;
		case "Backspace":
			buttonsHeld.select = true;
			break;
		case "x":
			buttonsHeld.b = true;
			break;
		case "z":
			buttonsHeld.a = true;
			break;
		case "ArrowDown":
			buttonsHeld.down = true;
			break;
		case "ArrowUp":
			buttonsHeld.up = true;
			break;
		case "ArrowLeft":
			buttonsHeld.left = true;
			break;
		case "ArrowRight":
			buttonsHeld.right = true;
	}
});

document.addEventListener("keyup", (event) => {
	switch (event.key) {
		case "Enter":
			buttonsHeld.start = false;
			break;
		case "Backspace":
			buttonsHeld.select = false;
			break;
		case "x":
			buttonsHeld.b = false;
			break;
		case "z":
			buttonsHeld.a = false;
			break;
		case "ArrowDown":
			buttonsHeld.down = false;
			break;
		case "ArrowUp":
			buttonsHeld.up = false;
			break;
		case "ArrowLeft":
			buttonsHeld.left = false;
			break;
		case "ArrowRight":
			buttonsHeld.right = false;
	}
});

