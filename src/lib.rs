#![no_std]

mod audio;
mod bootrom;
mod bus;
mod cpu;
#[macro_use]
mod wasm;

use crate::wasm::*;

pub struct GB {
    cpu: cpu::Cpu,
    bus: bus::Bus,
    frame_buf: [u8; 160 * 144 * 4],
    tick: u128,
}

impl GB {
    pub const fn new() -> GB {
        GB {
            cpu: cpu::Cpu::new(),
            bus: bus::Bus::new(&bootrom::DATA),
            frame_buf: [0; 160 * 144 * 4],
            tick: 0,
        }
    }
}

static mut GB_INSTANCE: GB = GB::new();

#[unsafe(no_mangle)]
pub fn setup() -> *mut GB {
    println!("emulator starting!");
    &raw mut GB_INSTANCE as *mut GB
}

#[unsafe(no_mangle)]
pub fn get_frame_buffer(gb: *mut GB) -> *mut [u8] {
    unsafe { &raw mut (&mut *gb).frame_buf }
}


#[unsafe(no_mangle)]
pub fn cycle(gb: *mut GB, count: usize) {
    unsafe {
        let gb = &mut *gb;
        for _ in 0..count {
            gb.cpu.cycle(&mut gb.bus);
            if gb.cpu.pc().get() > 0xB {
                println!("{}: {:?}", gb.tick, gb.cpu);
            }
            gb.tick += 1;
        }
    }
}
