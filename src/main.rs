mod bus;
mod reg;
mod cpu;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;

fn main() {
    let mut boot_rom_reader = BufReader::with_capacity(
        0x100,
        File::open("dmg.bin").expect("boot rom file not found"),
    );

    let mut ticks = 0;
    let mut running = true;
    let mut cpu = cpu::Cpu::new();
    let mut bus = bus::Bus::new(
        boot_rom_reader.fill_buf().unwrap()
            .try_into()
            .expect("Boot ROM must be exactly 256 bytes"),
    );

    while running {
        println!("{cpu:?}");
        cpu.cycle(&mut bus);
        ticks += 1;
        if ticks > 20 { running = false; }
    }
}
