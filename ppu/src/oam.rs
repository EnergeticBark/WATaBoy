const OBJ_SIZE: usize = 4;
const OAM_ADDR: usize = 0xFE00;

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
    fn intersects_y(&self, y: u8) -> bool {
        let screen_y_top = self.y_pos + 16;
        let screen_y_bottom = screen_y_top + 8;
        (screen_y_top..screen_y_bottom).contains(&y)
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
        .filter(|obj| obj.intersects_y(ly))
        .take(10)
        .collect()
}
