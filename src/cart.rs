use crate::bus;
use crate::gb;
use crate::*;

use core::cmp;

enum MbcType {
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

    ram_bank_offset: usize,
    rom_bank_offset: usize,

    mbc: MbcType,
    rom_bank_limit: u16,
    ram_bank_limit: u8,
}

impl Cart {
    const ROM_BANK_SIZE: usize = 0x4000;
    const RAM_BANK_SIZE: usize = 0x2000;

    pub const fn new() -> Cart {
        Cart {
            rom_image: [0; gb::MAX_CART_ROM_SIZE],
            ram: [0; 0x20000],
            ram_we: false,
            ram_bank_offset: 0,
            rom_bank_offset: Cart::ROM_BANK_SIZE,

            mbc: MbcType::RomOnly,
            ram_bank_limit: 0,
            rom_bank_limit: 0,
        }
    }

    fn parse_new_image(&mut self) {
        self.mbc = match self.rom_image[0x147] {
            0x03 => MbcType::Mbc1RamBattery,
            _ => todo!("cart type {}", self.rom_image[0x147]),
        };

        self.rom_bank_limit = match self.rom_image[0x148] {
            0x04 => 0x200,
            _ => todo!("cart rom bank {}", self.rom_image[0x148]),
        };

        self.ram_bank_limit = match self.rom_image[0x149] {
            0x03 => 4,
            _ => todo!("cart ram bank {}", self.rom_image[0x149]),
        };
    }


    pub fn read_rom(&self, addr: bus::Addr) -> u8 {
        match addr {
            0x0000..0x4000 => self.rom_image[addr as usize],
            0x4000..0x8000 => self.rom_image[self.rom_bank_offset + addr as usize],
            _ => unreachable!("{addr:04X}")
        }
    }

    pub fn write_rom(&mut self, addr: bus::Addr, val: u8) {
        match addr {
            0x2000..0x4000 => {
                self.rom_bank_offset = Cart::ROM_BANK_SIZE * cmp::min(val & 0x1F, 1) as usize;
            }
            _ => todo!("{addr:04X}, {val:02X}")
        }
    }

    pub fn read_ram(&self, addr: bus::Addr) -> u8 {
        todo!("{addr:04X}")
    }

    pub fn write_ram(&mut self, addr: bus::Addr, val: u8) {
        todo!("{addr:04X}, {val:02X}")
    }
}
