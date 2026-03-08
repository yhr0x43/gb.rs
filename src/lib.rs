#![no_std]

mod audio;
mod bus;
mod cart;
mod cpu;
mod gb;
mod graphic;
#[macro_use]
mod wasm;

use crate::wasm::*;

static mut GB_SINGLETON: gb::GB = gb::GB::new();

#[unsafe(no_mangle)]
pub fn gb_get() -> *mut gb::GB {
    println!("fetched GB instance");
    &raw mut GB_SINGLETON
}

#[unsafe(no_mangle)]
pub fn get_frame_buffer_ptr(gb: &gb::GB) -> *const u8 {
    gb.bus.ppu.frame_buffer.as_ptr()
}

#[unsafe(no_mangle)]
pub fn get_bootrom_ptr(gb: &mut gb::GB) -> *const u8 {
    gb.bus.bootrom.as_ptr()
}

#[unsafe(no_mangle)]
pub fn get_gamerom_ptr(gb: &mut gb::GB) -> *const u8 {
    gb.bus.cart.rom_image.as_ptr()
}

#[unsafe(no_mangle)]
pub fn run_cycles(gb: &mut gb::GB, count: usize) {
    let _ = (0..count).try_for_each(|_| gb.cycle());
}

#[unsafe(no_mangle)]
pub fn pause(gb: &mut gb::GB, val: i32) {
    gb.paused = val != 0;
}

#[unsafe(no_mangle)]
pub fn write_button_state(gb: &mut gb::GB, info: usize) {
    gb.write_button_state(info as u8); // TODO(yhr0x43): ????
}
