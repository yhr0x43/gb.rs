use crate::bus;
use crate::gb;

pub struct Ppu {
    pub(crate) frame_buffer: [u8; gb::FRAME_BUFFER_SIZE],
    write_pos: usize, // offset into frame_buffer

    pub(crate) tile_image: [u8; 0x40000],

    hdot: u16, // logical dot (progress) in one hline

    // internal Mode 3 states
    draw: bool,         // if we are in Mode 3
    lx: u8,             // physical dot on screen
    penalty: u8,        // counter for penalty simulation
    sc3_line: u8,       // low 3 bits of scx for this line
    tile_line: [u8; 8], // one hline of tile

    // memory/registers
    vram: [u8; 0x2000], // 8000..9FFF
    oam: [u8; 0xA0],    // FE00..FE9F

    pub(crate) lcdc: u8,
    pub(crate) stat: u8,
    pub(crate) scy: u8,
    pub(crate) scx: u8,
    pub(crate) ly: u8,
    pub(crate) lyc: u8,
    pub(crate) bgp: u8,
    pub(crate) obp0: u8,
    pub(crate) obp1: u8,
    pub(crate) wx: u8,
    pub(crate) wy: u8,
}

impl Ppu {
    const LCDC_ENABLE: u8 = 0x80;
    const LCDC_WN_MAP: u8 = 0x40;
    const LCDC_WN_ENABLE: u8 = 0x20;
    const LCDC_TILE_DATA: u8 = 0x10;
    const LCDC_BG_MAP: u8 = 0x08;
    const LCDC_OBJ_SIZE: u8 = 0x04;
    const LCDC_OBJ_ENABLE: u8 = 0x02;
    const LCDC_BGWN_PRIO: u8 = 0x01;

    pub const fn new() -> Ppu {
        Ppu {
            tile_image: [0; 0x40000],
            
            frame_buffer: [0; gb::FRAME_BUFFER_SIZE],
            write_pos: 0,
            draw: false,
            lx: 0,
            hdot: 0,
            penalty: 0,
            sc3_line: 0,
            tile_line: [0; 8],

            vram: [0; 0x2000],
            oam: [0; 0xA0],

            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wx: 0,
            wy: 0,
        }
    }

    // TODO(yhr0x43): memory locking
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

    pub fn tick(&mut self) {
        if self.lcdc & Ppu::LCDC_ENABLE == 0 {
            return;
        }

        self.dot();
        self.dot();
        self.dot();
        self.dot();
    }

    fn weave_bits(val: &[u8; 2]) -> [u8; 8] {
        [7, 6, 5, 4, 3, 2, 1, 0].map(|i| (val[0] & 1 << i) >> i | (val[1] & 1 << i) >> (i - 1))
    }

    fn fetch_tile(&self, map_x: u8, map_y: u8) -> [u8; 8] {
        let tile_x = (map_x as usize) / 8;
        let tile_y = (map_y as usize) / 8;

        let map_base = if self.lcdc & Ppu::LCDC_BG_MAP == 0 {
            0x1800
        } else {
            0x1C00
        };

        let tile_idx = self.vram[map_base + tile_x + tile_y * 0x20];

        let tile_data = if self.lcdc & Ppu::LCDC_TILE_DATA == 0 {
            (0x0800 + (tile_idx.cast_signed() as i16) * 0x10).cast_unsigned()
                + (map_y as u16 % 8) * 2
        } else {
            0x0000 + (tile_idx as u16) * 0x10 + (map_y % 8 * 2) as u16
        } as usize;

        Ppu::weave_bits(self.vram[tile_data..(tile_data+2)].try_into().unwrap())
    }

    // see https://gbdev.io/pandocs/Rendering.html for terminology
    // TODO(yhr0x43): vectorize dot
    fn dot(&mut self) {
        self.hdot += 1;
        if self.hdot > 455 {
            self.hdot = 0;
            self.ly += 1;
            if self.ly > 153 {
                self.ly = 0;
            }
        }

        if self.ly >= gb::FRAME_HEIGHT as u8 {
            return;
        }

        let map_x = self.lx.wrapping_add(self.scx);
        let map_y = self.ly.wrapping_add(self.scy);

        if self.hdot < 80 {
            if self.lcdc & Ppu::LCDC_OBJ_ENABLE != 0 {
                todo!("Object Rendering");
            }
            return;
        }

        // begin Mode 3
        if self.hdot == 80 {
            self.sc3_line = self.scx % 8;
            self.penalty = self.sc3_line;
            self.draw = true;

            self.tile_line = self.fetch_tile(map_x, map_y);
        }

        if self.penalty > 0 {
            self.penalty -= 1;
            return;
        }

        if self.draw {
            let tgt = (self.lx as usize + self.ly as usize * gb::FRAME_WIDTH) * 4;

            if map_x % 8 == 0 {
                self.tile_line = self.fetch_tile(map_x, map_y);
            }

            let (r, g, b, a) = Ppu::map_color(self.tile_line[(map_x % 8) as usize]);
            self.frame_buffer[tgt + 0] = r;
            self.frame_buffer[tgt + 1] = g;
            self.frame_buffer[tgt + 2] = b;
            self.frame_buffer[tgt + 3] = a;

            self.lx += 1;
            if self.lx >= gb::FRAME_WIDTH as u8 {
                self.draw = false;
                self.lx = 0;
            }
        }
        // end Mode 3
    }

    #[inline]
    fn map_color(i: u8) -> (u8, u8, u8, u8) {
        match i & 0x3 {
            0 => (0xFF, 0xFF, 0xFF, 0xFF),
            1 => (0xAA, 0xAA, 0xAA, 0xFF),
            2 => (0x55, 0x55, 0x55, 0xFF),
            3 => (0x00, 0x00, 0x00, 0xFF),
            _ => unreachable!(),
        }
    }

    pub fn put_tile_image(&mut self) {
        for y in 0..0x100 {
            for x in (0..0x100).step_by(8) {
                let tile = self.fetch_tile(x as u8, y as u8);
                for (ic, c) in tile.into_iter().enumerate() {
                    let (r, g, b, a) = Ppu::map_color(c);
                    self.tile_image[(ic + x + y * 0x100) * 4 + 0] = r;
                    self.tile_image[(ic + x + y * 0x100) * 4 + 1] = g;
                    self.tile_image[(ic + x + y * 0x100) * 4 + 2] = b;
                    self.tile_image[(ic + x + y * 0x100) * 4 + 3] = a;
                }
            }
        }
    }
}
