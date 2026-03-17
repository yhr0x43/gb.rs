use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::fmt::{Arguments, Write, Error};
use core::cell::Cell;
use core::ptr;
use core::arch::wasm32::{memory_size, memory_grow};

unsafe extern "C" {
    pub fn wasm_log(ptr: *const u8, len: usize);
}

#[macro_export]
macro_rules! println {
    ($($t:tt)*) => ( wrap_wasm_log(&format_args!($($t)*)) )
}

struct WasmWriter {
    buf: [u8; 0x100],
    len: usize,
}

impl WasmWriter {
    pub fn new() -> Self {
        WasmWriter { buf: [0; 0x100], len: 0 }
    }

    fn flush(&mut self) {
        unsafe {
            wasm_log(self.buf.as_ptr(), self.len);
        }
        self.len = 0;
    }
}

impl Drop for WasmWriter {
    fn drop(&mut self) {
        self.flush();
    }
}

impl Write for WasmWriter {
    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        let mut src = s;
        while self.len + src.len() > self.buf.len() {
            let usable = self.buf.len() - self.len;
            unsafe {
                ptr::copy_nonoverlapping(src.as_ptr(), self.buf.as_mut_ptr().add(self.len), usable);
            }
            self.flush();
            src = &src[usable..];
        }
        unsafe {
            ptr::copy_nonoverlapping(src.as_ptr(), self.buf.as_mut_ptr().add(self.len), src.len());
        }
        self.len += src.len();
        Ok(())
    }
}

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator { used: Cell::new(false) };
unsafe impl Sync for Allocator {}

pub struct Allocator {
    used: Cell<bool>,
}

const WASM_MEM_BLOCK_SIZE: usize = 0x10000; // 64 Ki

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if self.used.replace(true) {
            return ptr::null_mut()
        }

        let size = layout.size();

        if size == 0 {
            return ptr::null_mut()
        }

        let delta_pages = size.div_ceil(WASM_MEM_BLOCK_SIZE);
        let prev_pages = memory_grow(0, delta_pages);

        if prev_pages == usize::MAX {
            return ptr::null_mut();
        };

        let addr = prev_pages * WASM_MEM_BLOCK_SIZE;

        if addr % layout.align() != 0 {
            self.used.set(false);
            return ptr::null_mut();
        };

        addr as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
    }
}

pub fn wrap_wasm_log(value: &Arguments) {
    let mut w = WasmWriter::new();
    write!(&mut w, "{}", value).unwrap();
}


fn fmt_u32(mut val: u32, buf: &mut [u8; 10]) -> &str {
    let mut i = 0;
    let mut divisor = 1000000000;
    while i < 10 && divisor > 0 && val > 0{
        let quotient = val / divisor;
        if quotient != 0 {
            buf[i] = (quotient as u8) | 0x30;
            i += 1;
            val %= divisor;
        }
        divisor /= 10;
    }
    unsafe { str::from_utf8_unchecked(&buf[0..i]) }
}

#[panic_handler]
fn wasm_panic(info: &PanicInfo) -> ! {
    let mut w = WasmWriter::new();
    write!(w, "{}", info).unwrap_or_else(|_| {
        write!(w, "panic formatting failure").unwrap_or(());
    });
    drop(w);
    core::arch::wasm32::unreachable()
}

// fn wasm_panic(info: &PanicInfo) -> ! {
//     let mut w = WasmWriter::new();
//     let mut buf = [0; 10];
//     if let Some(loc) = info.location() {
//         let _ = w.write_str(loc.file());
//         let _ = w.write_str(":");
//         let _ = w.write_str(fmt_u32(loc.line(), &mut buf));
//         let _ = w.write_str(":");
//         let _ = w.write_str(fmt_u32(loc.column(), &mut buf));
//         let _ = w.write_str("\n");
//     }
//     if let Some(msg) = info.message().as_str() {
//         let _ = w.write_str(msg);
//     } else {
//         let _ = w.write_str("panic message formatting is not supported");
//     }
//     drop(w);
//     core::arch::wasm32::unreachable()
// }
