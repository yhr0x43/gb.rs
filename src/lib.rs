#![no_std]
mod bus;
mod cpu;
#[macro_use]
mod wasm;

mod bootrom;
use crate::wasm::*;


fn emu_run(boot_rom: &[u8; 0x100]) {
    wrap_wasm_log(&format_args!("emulator starting!"));

    let mut ticks: u64 = 0;
    let mut cpu = cpu::Cpu::new();
    let mut bus = bus::Bus::new(boot_rom);
    let mut running = true;

    while running {
        cpu.cycle(&mut bus);
        console_log!("{ticks:?}: {cpu:?}");
        ticks += 1;
        if ticks > 20 {
            running = false;
        }
    }
}

pub fn setup() {
}

pub fn cycle() {
}


#[unsafe(no_mangle)]
pub fn main() {
    emu_run(&bootrom::DATA);
}

#[unsafe(no_mangle)]
pub fn panic() {
    panic!("no")
}
