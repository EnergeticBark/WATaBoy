export const frametimeCounter = new class {
    constructor() {
		this.frametimes = document.querySelector("#frametimes");
		this.frames = [];
		this.frameStartTimestamp;
    }
  
    start() {
	    this.frameStartTimestamp = performance.now();
    }

    end() {
		const now = performance.now();
		const delta = now - this.frameStartTimestamp;
		
		// Save only the latest 100 timings.
		this.frames.push(delta);
		if (this.frames.length > 100) {
			this.frames.shift();
		}
		
		// Find the max, min, and mean of our 100 latest timings.
		let min = Infinity;
		let max = -Infinity;
		let sum = 0;
		for (let i = 0; i < this.frames.length; i++) {
			sum += this.frames[i];
			min = Math.min(this.frames[i], min);
			max = Math.max(this.frames[i], max);
		}
		let mean = sum / this.frames.length;
		
		this.frametimes.textContent = `
Frametimes: latest = ${Math.round(delta)}ms
avg of last 100 = ${Math.round(mean)}ms
min of last 100 = ${Math.round(min)}ms
max of last 100 = ${Math.round(max)}ms
`.trim();
  	}
};