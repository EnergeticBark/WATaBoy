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

// Prevent double-tap to zoom on mobile Safari.
document.addEventListener("dblclick", (event) => event.preventDefault());

const buttons = document.querySelector("#buttons");
buttons.addEventListener("touchstart", (event) => {
	if (!event.target.classList.contains("touch-button")) {
		return;
	}
	
	switch (event.target.id) {
		case "start-button":
			buttonsHeld.start = true;
			break;
		case "select-button":
			buttonsHeld.select = true;
			break;
		case "b-button":
			buttonsHeld.b = true;
			break;
		case "a-button":
			buttonsHeld.a = true;
			break;
		case "down-button":
			buttonsHeld.down = true;
			break;
		case "up-button":
			buttonsHeld.up = true;
			break;
		case "left-button":
			buttonsHeld.left = true;
			break;
		case "right-button":
			buttonsHeld.right = true;
	}
	
	event.target.style.backgroundColor = "green";
});

buttons.addEventListener("touchend", (event) => {
	if (!event.target.classList.contains("touch-button")) {
		return;
	}
	
	switch (event.target.id) {
		case "start-button":
			buttonsHeld.start = false;
			break;
		case "select-button":
			buttonsHeld.select = false;
			break;
		case "b-button":
			buttonsHeld.b = false;
			break;
		case "a-button":
			buttonsHeld.a = false;
			break;
		case "down-button":
			buttonsHeld.down = false;
			break;
		case "up-button":
			buttonsHeld.up = false;
			break;
		case "left-button":
			buttonsHeld.left = false;
			break;
		case "right-button":
			buttonsHeld.right = false;
	}
	
	event.target.style.backgroundColor = null;
});


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

