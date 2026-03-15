use bitfield_struct::bitfield;
use std::collections::VecDeque;

use hw_constants::{OAM_SIZE, OAM_START};

use super::registers::LcdControl;

const OBJ_SIZE: usize = 4;

#[bitfield(u8, order = Msb)]
pub struct Attributes {
    pub priority: bool,
    pub y_flip: bool,
    pub x_flip: bool,
    pub palette: bool,
    #[bits(4)]
    __: u8, // Padding
}

#[derive(Copy, Clone, Debug)]
pub struct Obj {
    pub y_pos: u8,
    pub x_pos: u8,
    pub tile_index: u8,
    pub attributes: Attributes,
}

impl Obj {
    fn from_bytes(bytes: [u8; OBJ_SIZE]) -> Self {
        Self {
            y_pos: bytes[0],
            x_pos: bytes[1],
            tile_index: bytes[2],
            attributes: Attributes::from_bits(bytes[3]),
        }
    }

    fn intersects_y(self, y: u8, obj_size: bool) -> bool {
        let obj_height = if obj_size { 16 } else { 8 };

        let y_top = self.y_pos;
        let y_bottom = y_top + obj_height;
        (y_top..y_bottom).contains(&(y + 16))
    }

    #[must_use]
    pub fn intersects_x(&self, x: u8) -> bool {
        let x_left = self.x_pos;
        let x_right = x_left.wrapping_add(8);
        (x_left..x_right).contains(&(x + 8))
    }
}

/// # Panics
///
/// Will panic if the provided index causes an out of bounds memory read.
#[must_use]
pub fn nth_obj(oam: &[u8; OAM_SIZE as usize], index: usize) -> Obj {
    let offset = index * OBJ_SIZE;
    let obj_start = (OAM_START as usize + offset) - OAM_START as usize;
    let obj_end = obj_start + OBJ_SIZE;
    let obj_bytes = oam[obj_start..obj_end].try_into().unwrap();
    Obj::from_bytes(obj_bytes)
}

// Place objects into a sorted queue so we can pop them in-order as we draw the scanline.
pub(super) fn oam_scan(
    obj_buffer: &mut VecDeque<Obj>,
    oam: &[u8; OAM_SIZE as usize],
    lcdc: LcdControl,
    ly: u8,
) {
    let obj_size = lcdc.obj_size();

    obj_buffer.clear();
    let (chunks, []) = oam.as_chunks() else {
        unreachable!()
    };
    chunks
        .iter()
        .map(|bytes| Obj::from_bytes(*bytes))
        .filter(|obj| (1..160).contains(&obj.y_pos) && obj.intersects_y(ly, obj_size))
        .take(10)
        .for_each(|obj| obj_buffer.push_back(obj));

    // This sort is stable, so "objects earlier in the OAM should have higher priority" still holds.
    obj_buffer.make_contiguous().sort_by_key(|obj| obj.x_pos);
}
