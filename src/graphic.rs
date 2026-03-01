use crate::bus;

pub struct Ppu {
    vram: [u8; 0x2000], // 8000..9FFF
    oam: [u8; 0xA0],    // FE00..FE9F
    rlcd: [u8; 0x6],    // FF40..FF46
    robj: [u8; 0x5],    // FF47..FF4C
}

impl Ppu {
    pub const fn new() -> Ppu {
        Ppu {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            rlcd: [0; 0x6],
            robj: [0; 0x5],
        }
    }

    pub fn read_vram(&self, addr: bus::Addr) -> u8 {
        self.vram[(addr as usize) - 0x8000]
    }

    pub fn write_vram(&mut self, addr: bus::Addr, val: u8) {
        self.vram[(addr as usize) - 0x8000] = val
    }

    pub fn read_oam(&self, addr: bus::Addr) -> u8 {
        self.oam[(addr as usize) - 0xFE00]
    }

    pub fn write_oam(&mut self, addr: bus::Addr, val: u8) {
        self.oam[(addr as usize) - 0xFE00] = val
    }

    pub fn read_regs(&self, addr: bus::Addr) -> u8 {
        match addr {
            0xFF40..0xFF46 => self.rlcd[(addr as usize) - 0xFF40],
            0xFF46..0xFF47 => todo!("oam dma read"),
            0xFF47..0xFF4C => self.robj[(addr as usize) - 0xFF47],
            _ => unreachable!("ppu reg read {:04X}", addr),
        }
    }

    pub fn write_regs(&mut self, addr: bus::Addr, val: u8) {
        match addr {
            0xFF40..0xFF46 => self.rlcd[(addr as usize) - 0xFF40] = val,
            0xFF46..0xFF47 => todo!("oam dma write"),
            0xFF47..0xFF4C => self.robj[(addr as usize) - 0xFF47] = val,
            _ => unreachable!("ppu reg write {:04X}", addr),
        }
    }
}
