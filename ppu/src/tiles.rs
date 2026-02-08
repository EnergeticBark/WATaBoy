use arbitrary_int::u2;

const TILE_SIZE: usize = 16;
const TILE_BLOCK_0_ADDR: usize = 0x8000;
const TILE_BLOCK_2_ADDR: usize = 0x9000;

const TILE_MAP_SIZE: usize = 0x0400;
const TILE_MAP_0_ADDR: usize = 0x9800;
const TILE_MAP_1_ADDR: usize = 0x9C00;

#[must_use]
pub fn tile_to_palette_indices(tile: &[u8; 16]) -> Vec<u2> {
    let (chunks, _) = tile.as_chunks::<2>();
    chunks
        .iter()
        .flat_map(|&[least_significant, most_significant]| {
            (0..8)
                .map(move |nth_bit| {
                    let mut palette_index = 0;
                    if least_significant >> nth_bit & 1 == 1 {
                        palette_index += 1;
                    }
                    if most_significant >> nth_bit & 1 == 1 {
                        palette_index += 2;
                    }

                    u2::new(palette_index)
                })
                .rev()
        })
        .collect()
}

#[must_use]
pub fn unsigned_nth_tile(memory: &[u8], tile_id: usize) -> &[u8; TILE_SIZE] {
    let tile_start = TILE_BLOCK_0_ADDR + tile_id * TILE_SIZE;
    let tile_end = tile_start + TILE_SIZE;
    memory[tile_start..tile_end].try_into().unwrap()
}

#[must_use]
pub fn signed_nth_tile(memory: &[u8], tile_id: isize) -> &[u8; TILE_SIZE] {
    let offset = tile_id * TILE_SIZE as isize;
    let tile_start = TILE_BLOCK_2_ADDR.wrapping_add_signed(offset);
    let tile_end = tile_start + TILE_SIZE;
    memory[tile_start..tile_end].try_into().unwrap()
}

#[must_use]
pub fn tile_map_0(memory: &[u8]) -> &[u8; TILE_MAP_SIZE] {
    let tile_map_start = TILE_MAP_0_ADDR;
    let tile_map_end = tile_map_start + TILE_MAP_SIZE;
    memory[tile_map_start..tile_map_end].try_into().unwrap()
}

#[must_use]
pub fn tile_map_1(memory: &[u8]) -> &[u8; TILE_MAP_SIZE] {
    let tile_map_start = TILE_MAP_1_ADDR;
    let tile_map_end = tile_map_start + TILE_MAP_SIZE;
    memory[tile_map_start..tile_map_end].try_into().unwrap()
}
