mod bus;
mod cpu;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

fn emu_run(boot_rom: &[u8; 0x100]) {
    let mut ticks: u64 = 0;
    let running = true;
    let mut cpu = cpu::Cpu::new();
    let mut bus = bus::Bus::new(boot_rom);

    while running {
        cpu.cycle(&mut bus);
        println!("{ticks:?}: {cpu:?}");
        ticks += 1;
    }
}

fn main() {
    let mut boot_rom_reader = BufReader::with_capacity(
        0x100,
        File::open("dmg.bin").expect("boot rom file not found"),
    );

    emu_run(
        boot_rom_reader
            .fill_buf()
            .unwrap()
            .try_into()
            .expect("Boot ROM must be exactly 256 bytes"),
    );
}
