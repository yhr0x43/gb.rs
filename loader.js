let app = document.getElementById("app");
let ctx = app.getContext("2d");

let memory = undefined;
const decoder = new TextDecoder('utf-8');

WebAssembly.instantiateStreaming(fetch('build/gb_rs.wasm'), {
    env: { "wasm_log": (ptr, len) => {
        const buffer = memory.buffer;
        console.log(decoder.decode(new Uint8Array(buffer, Number(ptr), Number(len))));
    }, }
}).then(w => {
    memory = w.instance.exports.memory;
    console.log(memory);
    w.instance.exports.main();
});
