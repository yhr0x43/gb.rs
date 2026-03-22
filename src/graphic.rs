use core::slice;

use crate::bus;
use crate::gb;

use crate::*;

struct ObjLine {
    ci: [u8; 8],
    x: u8,
    palette: bool,
}

#[derive(Clone, Copy)]
#[repr(packed)]
struct Obj {
    y: u8,
    x: u8,
    tile: u8,
    attr: u8,
}

impl Obj {
    const PRIORITY: u8 = 0x80;
    const YFLIP: u8 = 0x40;
    const XFLIP: u8 = 0x20;
    const DMG_PALETTE: u8 = 0x10;
    const BANK: u8 = 0x08;
    const CGB_PALETTE: u8 = 0x07;
}

pub struct Ppu {
    pub(crate) frame_buffer: [u8; gb::FRAME_BUFFER_SIZE],
    pub(crate) tile_image: [u8; 0x40000],

    hdot: u16, // logical dot (progress) in one hline

    // OBJ rendering states
    objs: [ObjLine; 10],
    obj_put: u8,
    obj_fetch: u8,

    // internal Mode 3 states
    draw: bool,   // if we are in Mode 3
    lx: u8,       // physical dot on screen
    penalty: u8,  // counter for penalty simulation
    sc3_line: u8, // low 3 bits of scx for this line

    // tile-specific state
    tile_line: [u8; 8],
    tile_obj_drawn: bool,

    // memory/registers
    vram: [u8; 0x2000], // 8000..9FFF
    oam: [Obj; 40],     // FE00..FE9F

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

    pub const fn init(&mut self) {}

    // TODO(yhr0x43): memory locking
    pub fn read_vram(&self, addr: bus::Addr) -> u8 {
        self.vram[(addr as usize) - 0x8000]
    }

    pub fn write_vram(&mut self, addr: bus::Addr, val: u8) {
        self.vram[(addr as usize) - 0x8000] = val
    }

    pub fn read_oam(&self, addr: bus::Addr) -> u8 {
        (unsafe { slice::from_raw_parts(self.oam.as_ptr() as *const u8, 160) })
            [(addr as usize) - 0xFE00]
    }

    pub fn write_oam(&mut self, addr: bus::Addr, val: u8) {
        (unsafe { slice::from_raw_parts_mut(self.oam.as_mut_ptr() as *mut u8, 160) })
            [(addr as usize) - 0xFE00] = val
    }

    pub fn tick(&mut self) -> bool {
        if self.lcdc & Ppu::LCDC_ENABLE == 0 {
            return false;
        }

        if self.dot() {
            return true;
        };
        if self.dot() {
            return true;
        };
        if self.dot() {
            return true;
        };
        if self.dot() {
            return true;
        };
        false
    }

    fn decode_2bpp(val: &[u8; 2], flip: bool) -> [u8; 8] {
        if flip {
            [0, 1, 2, 3, 4, 5, 6, 7]
        } else {
            [7, 6, 5, 4, 3, 2, 1, 0]
        }
        .map(|i| {
            (val[0] & 1 << i) >> i
                | if i == 0 {
                    (val[1] & 1) << 1
                } else {
                    (val[1] & (1 << i)) >> (i - 1)
                }
        })
    }

    fn fetch_tile(&self, map_base: usize, map_x: u8, map_y: u8) -> [u8; 8] {
        let tile_x = (map_x as usize) / 8;
        let tile_y = (map_y as usize) / 8;

        let tile_idx = self.vram[map_base + tile_x + tile_y * 0x20];

        let tile_addr = if self.lcdc & Ppu::LCDC_TILE_DATA == 0 {
            (0x1000 + (tile_idx.cast_signed() as i16) * 0x10).cast_unsigned()
        } else {
            0x0000 + (tile_idx as u16) * 0x10
        } as usize
            + ((map_y % 8) * 2) as usize;

        Ppu::decode_2bpp(
            self.vram[tile_addr..(tile_addr + 2)].try_into().unwrap(),
            false,
        )
    }

    // see https://gbdev.io/pandocs/Rendering.html for terminology
    // TODO(yhr0x43): vectorize dot
    // TODO(yhr0x43): burn-in the screen if LCDC is changed outside VBlank?
    fn dot(&mut self) -> bool {
        self.hdot += 1;
        if self.hdot > 455 {
            self.hdot = 0;
            self.lx = 0;
            self.ly += 1;
            if self.ly > 153 {
                self.ly = 0;
            }
        }

        // VBlank
        if self.ly >= gb::FRAME_HEIGHT as u8 {
            if self.ly == gb::FRAME_HEIGHT as u8 && self.hdot == 0 {
                return true;
            }
            return false;
        }

        // HBlank
        if self.lx >= gb::FRAME_WIDTH as u8 {
            return false;
        }

        if self.hdot == 0 {
            self.obj_put = 0;
            self.obj_fetch = 0;
        }

        if self.hdot < 80 {
            if self.lcdc & Ppu::LCDC_OBJ_ENABLE != 0 && self.hdot.is_multiple_of(8) {
                while self.obj_fetch < 40 {
                    let this_obj = &self.oam[self.obj_fetch as usize];
                    let obj_tall = self.lcdc & Ppu::LCDC_OBJ_SIZE != 0;
                    let mode_dy = if obj_tall { 16 } else { 8 };

                    if self.ly.wrapping_sub(this_obj.y.wrapping_sub(16)) < mode_dy {
                        let tile_y = if this_obj.attr & Obj::YFLIP != 0 {
                            mode_dy - self.ly % mode_dy
                        } else {
                            self.ly % mode_dy
                        };
                        let tile_addr = if obj_tall {
                            this_obj.tile & 0xFE
                        } else {
                            this_obj.tile
                        } as usize
                            * 0x10
                            + 2 * tile_y as usize;

                        let tiles = &self.vram[tile_addr..tile_addr + 2];

                        self.objs[self.obj_put as usize] = ObjLine {
                            ci: Ppu::decode_2bpp(
                                tiles.try_into().unwrap(),
                                this_obj.attr & Obj::XFLIP != 0,
                            ),
                            x: this_obj.x,
                            palette: this_obj.attr & Obj::DMG_PALETTE != 0,
                        };
                        self.obj_put += 1;
                        self.obj_fetch += 1;
                        return false;
                    }
                    self.obj_fetch += 1;
                }
            }
            return false;
        }

        // begin Mode 3
        if self.hdot == 80 {
            self.sc3_line = self.scx % 8;
            self.penalty = self.sc3_line;
        }

        if self.penalty > 0 {
            self.penalty -= 1;
            return false;
        }

        let obj_color = if self.lcdc & Ppu::LCDC_OBJ_ENABLE != 0 {
            // TODO object occlusion
            let mut color = 0x00;
            for obj in &self.objs[..self.obj_put as usize] {
                let dx = self.lx.wrapping_sub(obj.x.wrapping_sub(8));
                if dx < 8 {
                    self.tile_obj_drawn = true;
                    if obj.ci[dx as usize] != 0x00 {
                        let pal = if obj.palette { self.obp0 } else { self.obp1 };
                        let ci = obj.ci[dx as usize];
                        color = (pal & (0x3 << (ci * 2))) >> ci * 2;
                        break;
                    }
                }
            }
            color
        } else {
            0x00
        };

        let bg_color = if self.lcdc & Ppu::LCDC_BGWN_PRIO != 0 {
            let window_color = if self.lcdc & Ppu::LCDC_WN_ENABLE != 0 {
                let wm_map_x = self.lx.wrapping_sub(self.wx);
                let wm_map_y = self.ly.wrapping_sub(self.wy);

                if self.lx < self.wx || self.ly < self.wy {
                    if self.lx + 1 == self.wx {
                        self.penalty = 6;
                        return false;
                    };
                    0x00
                } else if wm_map_x < 160 && wm_map_y < 144 {
                    let map_base = if self.lcdc & Ppu::LCDC_WN_MAP == 0 {
                        0x1800
                    } else {
                        0x1C00
                    };

                    let tile_line = self
                        .fetch_tile(map_base, wm_map_x, wm_map_y)
                        .map(|ci| (self.bgp & 0x3 << ci * 2) >> ci * 2);
                    tile_line[(wm_map_x % 8) as usize]
                } else {
                    0x00
                }
            } else {
                0x00
            };

            if window_color != 0x00 {
                window_color
            } else {
                let map_x = self.lx.wrapping_add(self.scx & 0xF8 | self.sc3_line);
                let map_y = self.ly.wrapping_add(self.scy);

                if self.lx == 0 || map_x.is_multiple_of(8) {
                    let map_base = if self.lcdc & Ppu::LCDC_BG_MAP == 0 {
                        0x1800
                    } else {
                        0x1C00
                    };

                    self.tile_line = self
                        .fetch_tile(map_base, map_x, map_y)
                        .map(|ci| (self.bgp & 0x3 << ci * 2) >> ci * 2);
                }

                self.tile_line[map_x as usize % 8]
            }
        } else {
            0x00
        };

        let tgt = (self.lx as usize + self.ly as usize * gb::FRAME_WIDTH) * 4;
        if obj_color != 0 {
            self.frame_buffer[tgt..tgt + 4].copy_from_slice(&Ppu::map_color(obj_color));
        } else {
            self.frame_buffer[tgt..tgt + 4].copy_from_slice(&Ppu::map_color(bg_color));
        }

        self.lx += 1;
        // end Mode 3
        false
    }

    #[inline]
    fn map_color(i: u8) -> [u8; 4] {
        // match i & 0x3 {
        //     0 => [0x84, 0x96, 0x00, 0xFF],
        //     1 => [0x4A, 0x69, 0x00, 0xFF],
        //     2 => [0x29, 0x55, 0x00, 0xFF],
        //     3 => [0x10, 0x41, 0x00, 0xFF],
        //     _ => unreachable!(),
        // }
        match i & 0x3 {
            0 => [0xFF, 0xFF, 0xFF, 0xFF],
            1 => [0xAA, 0xAA, 0xAA, 0xFF],
            2 => [0x55, 0x55, 0x55, 0xFF],
            3 => [0x00, 0x00, 0x00, 0xFF],
            _ => unreachable!(),
        }
    }

    pub fn put_tile_image(&mut self) {
        for i in (0..0x10000).step_by(8) {
            let map_base = if self.lcdc & Ppu::LCDC_BG_MAP == 0 {
                0x1800
            } else {
                0x1C00
            };

            let tile = self.fetch_tile(map_base, (i & 0xFF) as u8, (i >> 8) as u8);
            for (ic, c) in tile.into_iter().enumerate() {
                let tgt = (ic + i) * 4;
                self.tile_image[tgt..tgt + 4].copy_from_slice(&Ppu::map_color(c));
            }
        }
    }
}
