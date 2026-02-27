unsafe extern "C" {
    fn wasm_log(ptr: *const u8, len: usize);
}

pub fn wrap_wasm_log(value: &String) {
    let value = value.as_str();
    unsafe {
        wasm_log(value.as_ptr(), value.len())
    }
}

#[macro_export] macro_rules! console_log {
    ($($t:tt)*) => ( wrap_wasm_log(&format_args!($($t)*).to_string()) )
}
