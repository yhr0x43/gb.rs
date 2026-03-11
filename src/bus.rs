use crate::audio::Apu;
use crate::cart::Cart;
use crate::graphic::Ppu;
use crate::intr::Intr;
use crate::timer::Timer;

pub(crate) type Addr = u16;

pub(crate) struct Bus {
    pub(crate) bootrom: [u8; 0x100],
    pub(crate) apu: Apu,
    pub(crate) ppu: Ppu,
    pub(crate) cart: Cart,
    pub(crate) intr: Intr,
    pub(crate) timer: Timer,

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
            intr: Intr::new(),
            timer: Timer::new(),

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
            0xE000..0xFE00 => self.wram[(addr as usize) - 0xE000],
            0xFE00..0xFEA0 => self.ppu.read_oam(addr),
            0xFEA0..0xFF00 => 0xFF, /* Not Used */
            0xFF00 => self.read_joystate(),
            0xFF01..0xFF03 => 0xFF, /* TODO: serial */
            0xFF03 => 0xFF, /* Unused */
            0xFF04 => self.timer.div,
            0xFF05 => self.timer.tima,
            0xFF06 => self.timer.tma,
            0xFF07 => self.timer.tac,
            0xFF08..0xFF0F => 0xFF, /* Unused */
            0xFF0F => self.intr.read_if(),
            0xFF10..0xFF40 => self.apu.read(addr),
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.stat,
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF46 => todo!("oam dma"),
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wx,
            0xFF4B => self.ppu.wy,
            0xFF4C..0xFF50 => 0xFF, /* DMG Not Used */
            0xFF50 => todo!("bootrom unmap register read behavior?"),
            0xFF51..0xFF80 => 0xFF, /* DMG Not Used */
            0xFF80..0xFFFF => self.hram[(addr as usize) - 0xFF80],
            0xFFFF => self.intr.read_ie(),
        }
    }

    pub fn write(&mut self, addr: Addr, val: u8) {
        match addr {
            0x0000..0x8000 => self.cart.write_rom(addr, val),
            0x8000..0xA000 => self.ppu.write_vram(addr, val),
            0xA000..0xC000 => self.cart.write_ram(addr, val),
            0xC000..0xE000 => self.wram[(addr as usize) - 0xC000] = val,
            0xE000..0xFE00 => self.wram[(addr as usize) - 0xE000] = val,
            0xFE00..0xFEA0 => self.ppu.write_oam(addr, val),
            0xFEA0..0xFF00 => { }, /* Not Used */
            0xFF00 => self.joy_sel = val,
            0xFF01..0xFF03 => { }, /* TODO: serial */
            0xFF03 => { }, /* Unused */
            0xFF04 => self.timer.div = 0,
            0xFF05 => self.timer.tima = val,
            0xFF06 => self.timer.tma = val,
            0xFF07 => self.timer.tac = val & 0x03,
            0xFF08..0xFF0F => { }, /* Unused */
            0xFF0F => self.intr.write_if(val),
            0xFF10..0xFF40 => self.apu.write(addr, val),
            0xFF40 => self.ppu.lcdc = val,
            0xFF41 => self.ppu.stat = val,
            0xFF42 => self.ppu.scy = val,
            0xFF43 => self.ppu.scx = val,
            0xFF44 => self.ppu.ly = val,
            0xFF45 => self.ppu.lyc = val,
            0xFF46 => todo!("oam dma: {:02X}", val),
            0xFF47 => self.ppu.bgp = val,
            0xFF48 => self.ppu.obp0 = val,
            0xFF49 => self.ppu.obp1 = val,
            0xFF4A => self.ppu.wx = val,
            0xFF4B => self.ppu.wy = val,
            0xFF4C..0xFF50 => { }, /* DMG Not Used */
            0xFF50 => self.boot_map = false,
            0xFF51..0xFF80 => { }, /* DMG Not Used */
            0xFF80..0xFFFF => self.hram[(addr as usize) - 0xFF80] = val,
            0xFFFF => self.intr.write_ie(val),
        };
    }
}
