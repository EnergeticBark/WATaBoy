use super::lcd_control::obj_size;

use std::collections::VecDeque;

use hw_constants::MEM_MAP_SIZE;

const OBJ_SIZE: usize = 4;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Obj {
    pub y_pos: u8,
    pub x_pos: u8,
    pub tile_index: u8,
    pub attributes: u8,
}

impl Obj {
    #[must_use]
    pub fn from_bytes(bytes: &[u8; OBJ_SIZE]) -> Self {
        Self {
            y_pos: bytes[0],
            x_pos: bytes[1],
            tile_index: bytes[2],
            attributes: bytes[3],
        }
    }

    #[must_use]
    pub fn priority(&self) -> bool {
        self.attributes & 0b1000_0000 == 0b1000_0000
    }
    #[must_use]
    pub fn y_flip(&self) -> bool {
        self.attributes & 0b0100_0000 == 0b0100_0000
    }
    #[must_use]
    pub fn x_flip(&self) -> bool {
        self.attributes & 0b0010_0000 == 0b0010_0000
    }
    #[must_use]
    pub fn palette(&self) -> bool {
        self.attributes & 0b0001_0000 == 0b0001_0000
    }

    #[must_use]
    pub fn intersects_y(&self, y: u8, obj_size: bool) -> bool {
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
pub fn nth_obj(memory: &[u8; MEM_MAP_SIZE], index: usize) -> Obj {
    let offset = index * OBJ_SIZE;
    let obj_start = hw_constants::OAM as usize + offset;
    let obj_end = obj_start + OBJ_SIZE;
    let obj_bytes = memory[obj_start..obj_end].try_into().unwrap();
    Obj::from_bytes(obj_bytes)
}

// Place objects into a sorted queue so we can pop them in-order as we draw the scanline.
pub fn oam_scan(obj_buffer: &mut VecDeque<Obj>, memory: &[u8; MEM_MAP_SIZE], ly: u8) {
    let obj_size = obj_size(memory);

    obj_buffer.clear();
    for index in 0..40 {
        let obj = nth_obj(memory, index);
        // Only consider objects on screen (y value between 1 and 160).
        if (1..160).contains(&obj.y_pos) && obj.intersects_y(ly, obj_size) {
            obj_buffer.push_back(obj);
            if obj_buffer.len() == 10 {
                break;
            }
        }
    }

    // This sort is stable, so "objects earlier in the OAM should have higher priority" still holds.
    obj_buffer.make_contiguous().sort_by_key(|obj| obj.x_pos);
}
