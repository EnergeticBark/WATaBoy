const utf8decoder = new TextDecoder();
export const console_log_glue = (stringPtr, stringLen) => {
	const messageBytes = new Uint8Array(instance.exports.memory.buffer, stringPtr, stringLen);
	const message = utf8decoder.decode(messageBytes);
	console.log(message);
}