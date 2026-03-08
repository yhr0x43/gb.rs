use crate::bus;
use crate::gb;


enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc2,
    Mbc2Battery,
    RomRam,
    RomRamBattery,
    Mmm01,
    Mmm01Ram,
    Mmm01RamBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery,
    Mbc3,
    Mbc3Ram,
    Mbc3RamBattery,
    Mbc5,
    Mbc5Ram,
    Mbc5RamBattery,
    Mbc5Rumble,
    Mbc5RumbleRam,
    Mbc5RumbleRamBattery,
    Mbc6,
    Mbc7SensorRumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1RamBattery,
}

pub(crate) struct Cart {
    pub rom_image: [u8; gb::MAX_CART_ROM_SIZE],

    ram: [u8; 0x20000],
    ram_we: bool,

    ram_bank: u8,
    rom_bank: u16,
}

impl Cart {
    const fn parse_cart_type(file: &[u8]) -> Option<CartridgeType> {
        todo!()
    }

    pub const fn new() -> Cart {
        Cart {
            rom_image: [0; gb::MAX_CART_ROM_SIZE],
            ram: [0; 0x20000],
            ram_we: false,
            ram_bank: 0,
            rom_bank: 0,
        }
    }

    pub fn read_rom(&self, addr: bus::Addr) -> u8 {
        match addr {
            0x0000..0x4000 => self.rom_image[addr as usize],
            0x4000..0x8000 => todo!("{addr}"),
            _ => unreachable!("{addr}")
        }
    }

    pub fn write_rom(&mut self, addr: bus::Addr, val: u8) {
        todo!("{addr:04X}, {val:02X}")
    }

    pub fn read_ram(&self, addr: bus::Addr) -> u8 {
        todo!("{addr:04X}")
    }

    pub fn write_ram(&mut self, addr: bus::Addr, val: u8) {
        todo!("{addr:04X}, {val:02X}")
    }
}
