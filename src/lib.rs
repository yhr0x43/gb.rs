#![no_std]
#![allow(unused)]

mod audio;
mod bus;
mod cart;
mod cpu;
mod gb;
mod graphic;
mod intr;
mod timer;
mod reg;
#[macro_use]
mod wasm;

use core::alloc::{GlobalAlloc, Layout};
use core::mem::{MaybeUninit, forget};
use core::ptr;

use crate::wasm::*;

#[unsafe(no_mangle)]
pub fn gb_get() -> *mut gb::GB {
    // static mut GB_INSTANCE: gb::GB = gb::GB::new();
    // unsafe { &raw mut GB_INSTANCE }
    // println!("begin gb_get");
    let layout = Layout::new::<gb::GB>();
    unsafe {
        let gb_ptr = ALLOCATOR.alloc_zeroed(layout) as *mut gb::GB;
        if let Some(gb) = gb_ptr.as_mut() {
            gb.init()
        }
        gb_ptr
    }
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
pub fn run_frame(gb: &mut gb::GB, count: usize) {
    let _ = (0..count).try_for_each(|_| gb.tick());
    gb.bus.ppu.put_tile_image();
}

#[unsafe(no_mangle)]
pub fn get_tile_image_ptr(gb: &gb::GB) -> *const u8 {
    gb.bus.ppu.tile_image.as_ptr()
}

#[unsafe(no_mangle)]
pub fn pause(gb: &mut gb::GB, val: i32) {
    gb.paused = val != 0;
    // println!("{:?}", gb.cpu);
}

#[unsafe(no_mangle)]
pub fn write_button_state(gb: &mut gb::GB, info: usize) {
    gb.write_button_state(info as u8); // TODO(yhr0x43): ????
}
