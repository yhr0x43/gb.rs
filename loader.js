let app = document.getElementById("app");
let ctx = app.getContext("2d");

let memory = undefined;
const encoder = new TextEncoder('utf-8');
const decoder = new TextDecoder('utf-8');

let bootptr = 0;

const wasm_env = {
    wasm_log: (ptr, len) => {
        console.log(decoder.decode(new Uint8Array(memory.buffer, Number(ptr), Number(len))));
    },
    wasm_never: (code) => {
        console.error("wasm_never: ", code);
    },
    boot_rom: (ptr) => {
        bootptr = ptr;
    }
}

WebAssembly.compileStreaming(fetch('build/gb_rs.wasm'))
    .then(module => {
        console.log(module);
        return WebAssembly.instantiate(module, {
            env: wasm_env,
            // env: new Proxy(wasm_env, {
            //     get(target, prop, receiver) {
            //         return prop in target ?
            //             target[prop] :
            //             (...args) => console.error("NOT IMPLEMENTED: " + prop, args);
            //     },
            // })
        });
    })
    .then(instance => {
        memory = instance.exports.memory;
        console.log(instance.exports);
        
        // function draw(timestamp) {
        //     const pixels = new Uint8ClampedArray(memory.buffer, 0, app.width * app.height * 4);
        //     ctx.putImageData(new ImageData(pixels, app.width, app.height), 0, 0);
        //     requestAnimationFrame(draw);
        // }
        // requestAnimationFrame(draw);

        //instance.exports.panic();
        instance.exports.main();

    });
