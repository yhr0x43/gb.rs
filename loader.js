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
        debugger;
    },
    boot_rom: (ptr) => {
        bootptr = ptr;
    }
}

WebAssembly.compileStreaming(fetch('build/gb_rs.wasm'))
    .then(module => {
        console.log(module);
        return WebAssembly.instantiate(module, {
            env: new Proxy(wasm_env, {
                get(target, prop, receiver) {
                    return prop in target ?
                        target[prop] :
                        (...args) => console.error("NOT IMPLEMENTED: " + prop, args);
                },
            })
        });
    })
    .then(instance => {
        memory = instance.exports.memory;
        console.log(instance.exports);
        //debugger;

        instance.exports.setup();
        // let ptr_frame_buffer = instance.exports.get_frame_buffer();
        let ptr_frame_buffer = 0;

        let gb = instance.exports.setup();
        instance.exports.cycle(gb, 1000000);
        
        function draw(timestamp) {
            const pixels = new Uint8ClampedArray(memory.buffer, ptr_frame_buffer, app.width * app.height * 4);
            ctx.putImageData(new ImageData(pixels, app.width, app.height), 0, 0);
            requestAnimationFrame(draw);
        }
        requestAnimationFrame(draw);
    });
