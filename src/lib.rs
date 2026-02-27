mod bus;
mod cpu;

#[macro_use]
pub mod log;

mod bootrom;

use crate::log::*;

fn emu_run(boot_rom: &[u8; 0x100]) {
    let mut ticks: u64 = 0;
    let mut running = true;
    let mut cpu = cpu::Cpu::new();
    let mut bus = bus::Bus::new(boot_rom);

    while running {
        cpu.cycle(&mut bus);
        console_log!("{ticks:?}: {cpu:?}");
        ticks += 1;
        if ticks > 20 {
            running = false;
        }
    }
}


#[unsafe(no_mangle)]
pub fn main() {
    emu_run(&bootrom::DATA);
}
