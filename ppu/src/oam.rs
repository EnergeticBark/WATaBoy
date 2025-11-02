const SPRITE_SIZE: usize = 4;
const OAM_ADDR: usize = 0xFE00;

pub fn nth_sprite(memory: &[u8], index: usize) -> &[u8; SPRITE_SIZE] {
    let offset = index * SPRITE_SIZE;
    let sprite_start = OAM_ADDR + offset;
    let sprite_end = sprite_start + SPRITE_SIZE;
    memory[sprite_start..sprite_end].try_into().unwrap()
}

pub fn y_pos(sprite: &[u8; SPRITE_SIZE]) -> u8 {
    sprite[0]
}

pub fn x_pos(sprite: &[u8; SPRITE_SIZE]) -> u8 {
    sprite[1]
}

pub fn tile_index(sprite: &[u8; SPRITE_SIZE]) -> u8 {
    sprite[2]
}

pub fn attributes(sprite: &[u8; SPRITE_SIZE]) -> u8 {
    sprite[3]
}