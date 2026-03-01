#![no_std]
mod bus;
mod cpu;
#[macro_use]
mod wasm;

mod bootrom;

use crate::wasm::*;

pub struct GB {
    cpu: cpu::Cpu,
    bus: bus::Bus,
    tick: u128,
}

impl GB {
    pub const fn new() -> GB {
        GB {
            cpu: cpu::Cpu::new(),
            bus: bus::Bus::new(&bootrom::DATA),
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
pub fn cycle(gb: *mut GB, count: usize) {
    unsafe {
        let gb = &mut *gb;
        for _ in 0..count {
            gb.cpu.cycle(&mut gb.bus);
            //println!("{ticks:3}: {cpu:?}");
            if gb.cpu.hl().get() < 0x8003 {
                println!("{}: {:?}", gb.tick, gb.cpu);
            }
        }
    }
}
