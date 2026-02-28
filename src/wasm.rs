use core::panic::PanicInfo;
use core::fmt::{Arguments, Write, Error};
use core::ptr::copy_nonoverlapping;


unsafe extern "C" {
    pub fn wasm_log(ptr: *const u8, len: usize);
    pub fn wasm_never(exit_code: usize) -> !;
}

struct WasmWriter {
    buf: [u8; 0x200],
    len: usize,
}

impl WasmWriter {
    pub fn new() -> Self {
        WasmWriter { buf: [0; 0x200], len: 0 }
    }

    pub fn flush(&mut self) {
        unsafe {
            wasm_log(self.buf.as_ptr(), self.len);
        }
        self.len = 0;
    }
}

impl Write for WasmWriter {
    // Required method
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let mut src = s;
        while self.len + src.len() > self.buf.len() {
            let usable = self.buf.len() - self.len;
            unsafe {
                copy_nonoverlapping(src.as_ptr(), self.buf.as_mut_ptr().add(self.len), usable);
            }
            self.flush();
            src = &src[usable..];
        }
        unsafe {
            copy_nonoverlapping(src.as_ptr(), self.buf.as_mut_ptr().add(self.len), src.len());
        }
        self.len += src.len();
        Ok(())
    }
}

pub fn wrap_wasm_log(value: &Arguments) {
    let mut w = WasmWriter::new();
    write!(&mut w, "{}", value).unwrap();
    w.flush();
}

#[macro_export]
macro_rules! println {
    ($($t:tt)*) => ( wrap_wasm_log(&format_args!($($t)*)) )
}

#[panic_handler]
fn wasm_panic(info: &PanicInfo) -> ! {
    let mut w = WasmWriter::new();
    write!(w, "{}", info).unwrap_or_else(|_| {
        write!(w, "panic formatting failure").unwrap_or(());
    });
    w.flush();
    unsafe { wasm_never(0) }
}
