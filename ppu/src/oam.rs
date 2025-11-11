const OBJ_SIZE: usize = 4;
const OAM_ADDR: usize = 0xFE00;

#[derive(Copy, Clone, Debug)]
pub struct Obj {
    pub y_pos: u8,
    pub x_pos: u8,
    pub tile_index: u8,
    pub attributes: u8,
}

impl Obj {
    pub fn from_bytes(bytes: &[u8; OBJ_SIZE]) -> Self {
        Self {
            y_pos: bytes[0],
            x_pos: bytes[1],
            tile_index: bytes[2],
            attributes: bytes[3],
        }
    }

    // TODO: handle 8x16 tile mode, this only handles 8x8.
    pub fn intersects_y(&self, y: u8) -> bool {
        let y_top = self.y_pos;
        let y_bottom = y_top + 8;
        (y_top..y_bottom).contains(&(y + 16))
    }

    pub fn intersects_x(&self, x: u8) -> bool {
        let x_left = self.x_pos;
        let x_right = x_left + 8;
        (x_left..x_right).contains(&(x + 8))
    }
}

pub fn nth_obj(memory: &[u8], index: usize) -> Obj {
    let offset = index * OBJ_SIZE;
    let obj_start = OAM_ADDR + offset;
    let obj_end = obj_start + OBJ_SIZE;
    let obj_bytes = memory[obj_start..obj_end].try_into().unwrap();
    Obj::from_bytes(obj_bytes)
}

pub fn oam_scan(memory: &[u8], ly: u8) -> Vec<Obj> {
    (0..40)
        .map(|index| nth_obj(memory, index))
        // Only consider objects on screen (y value between 1 and 160).
        .filter(|obj| (1..160).contains(&obj.y_pos))
        .filter(|obj| obj.intersects_y(ly))
        .take(10)
        .collect()
}
