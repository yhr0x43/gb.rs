use crate::bus;
use crate::gb;

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

    pub bank2: u8,
    pub bank4: u8,

    mbc: MbcType,
    rom_bank_limit: u8,
    ram_bank_limit: u8,

    mbc1mode: bool,
}

impl Cart {
    const ROM_BANK_SIZE: usize = 0x4000;
    const RAM_BANK_SIZE: usize = 0x2000;

    pub const fn init(&mut self) {
        self.mbc = MbcType::RomOnly;
        self.bank2 = 1;
    }

    fn parse_new_image(&mut self) {
        self.mbc = match self.rom_image[0x147] {
            0x03 => MbcType::Mbc1RamBattery,
            _ => todo!("cart type {}", self.rom_image[0x147]),
        };

        self.rom_bank_limit = match self.rom_image[0x148] {
            0x04 => 0x40,
            _ => todo!("cart rom bank {}", self.rom_image[0x148]),
        };

        self.ram_bank_limit = match self.rom_image[0x149] {
            0x03 => 0x04,
            _ => todo!("cart ram bank {}", self.rom_image[0x149]),
        };
    }

    fn low_rom_bank(&self) -> usize {
        if self.mbc1mode {
            self.bank4 << 5
        } else {
            0
        }.into()
    }

    fn high_rom_bank(&self) -> usize {
        (self.bank2 | self.bank4 << 5).into()
    }

    fn ram_bank(&self) -> usize {
        if self.mbc1mode {
            self.bank4 << 5
        } else {
            0
        }.into()
    }

    pub fn read_rom(&self, addr: bus::Addr) -> u8 {
        match addr {
            0x0000..0x4000 => self.rom_image[self.low_rom_bank() * Cart::ROM_BANK_SIZE | addr as usize],
            0x4000..0x8000 => self.rom_image[self.high_rom_bank() * Cart::ROM_BANK_SIZE + (addr - 0x4000) as usize],
            _ => unreachable!("{addr:04X}")
        }
    }

    pub fn write_rom(&mut self, addr: bus::Addr, val: u8) {
        match addr {
            0x0000..0x2000 => {
                self.ram_we = val & 0x0F == 0xA;
            }
            0x2000..0x4000 => {
                self.bank2 = cmp::max(val & 0x1F, 1);
            }
            0x4000..0x6000 => {
                self.bank4 = val & 0x03;
            }
            0x6000..0x8000 => {
                self.mbc1mode = val & 0x01 != 0;
            }
            _ => unreachable!()
        }
    }

    pub fn read_ram(&self, addr: bus::Addr) -> u8 {
        let offset: usize = (addr - 0xA000).into();
        self.ram[self.ram_bank() * Cart::RAM_BANK_SIZE + offset]
    }

    pub fn write_ram(&mut self, addr: bus::Addr, val: u8) {
        if self.ram_we {
            let offset: usize = (addr - 0xA000).into();
            self.ram[self.ram_bank() * Cart::RAM_BANK_SIZE + offset] = val;
        }
    }
}
