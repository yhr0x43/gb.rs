use crate::audio::Apu;
use crate::cart::Cart;
use crate::graphic::Ppu;

pub(crate) type Addr = u16;

pub(crate) struct Bus {
    pub(crate) bootrom: [u8; 0x100],
    pub(crate) apu: Apu,
    pub(crate) ppu: Ppu,
    pub(crate) cart: Cart,

    joy_state: u8,
    joy_sel: u8,

    wram: [u8; 0x2000],
    hram: [u8; 0x7F],

    boot_map: bool,
}

impl Bus {
    pub const fn new() -> Self {
        Bus {
            bootrom: [0; 0x100],

            wram: [0; 0x2000],
            hram: [0; 0x7F],

            apu: Apu::new(),
            ppu: Ppu::new(),
            cart: Cart::new(),

            boot_map: true,

            // hi: A, B, Select, Start. 0 = Pressed; 1 = Not pressed.
            // lo: Right, Left, Up, Down. 0 = Pressed; 1 = Not pressed.
            joy_state: 0xFF,
            joy_sel: 0x00,
        }
    }

    pub fn write_joystate(&mut self, state: u8) {
        self.joy_state = state;
    }

    pub fn read_joystate(&self) -> u8 {
        !(self.joy_state >> (4 * (self.joy_sel - 1))) & 0x0F | self.joy_sel
    }

    pub fn read(&self, addr: Addr) -> u8 {
        match addr {
            0x0000..0x0100 => {
                if self.boot_map {
                    self.bootrom[addr as usize]
                } else {
                    self.cart.read_rom(addr)
                }
            }
            0x0100..0x8000 => self.cart.read_rom(addr),
            0x8000..0xA000 => self.ppu.read_vram(addr),
            0xA000..0xC000 => self.cart.read_ram(addr),
            0xC000..0xE000 => self.wram[(addr as usize) - 0xC000],
            0xFE00..0xFEA0 => self.ppu.read_oam(addr),
            0xFEA0..0xFF00 => 0xFF, /* Not Used */
            0xFF00 => self.read_joystate(),
            0xFF10..0xFF40 => self.apu.read(addr),
            0xFF40..0xFF4B => self.ppu.read_regs(addr),
            0xFF51..0xFF80 => 0xFF, /* DMG Not Used */
            0xFF80..0xFFFF => self.hram[(addr as usize) - 0xFF80],
            0xFFFF => todo!("ie register read"),
            _ => todo!("memory address read {:04X}", addr),
        }
    }

    pub fn write(&mut self, addr: Addr, val: u8) {
        match addr {
            0x0000..0x8000 => self.cart.write_rom(addr, val),
            0x8000..0xA000 => self.ppu.write_vram(addr, val),
            0xA000..0xC000 => self.cart.write_ram(addr, val),
            0xC000..0xE000 => self.wram[(addr as usize) - 0xC000] = val,
            0xFE00..0xFEA0 => self.ppu.write_oam(addr, val),
            0xFEA0..0xFF00 => { }, /* Not Used */
            0xFF00 => self.joy_sel = val,
            0xFF10..0xFF40 => self.apu.write(addr, val),
            0xFF40..0xFF4B => self.ppu.write_regs(addr, val),
            0xFF50 => self.boot_map = false,
            0xFF51..0xFF80 => { }, /* DMG Not Used */
            0xFF80..0xFFFF => self.hram[(addr as usize) - 0xFF80] = val,
            0xFFFF => todo!("ie register write"),
            _ => todo!("memory address write {:04X}", addr),
        };
    }
}
