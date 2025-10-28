use std::ops::{Index, IndexMut, Range};


const MEM_MAP_SIZE: usize = 0x10000;

pub struct AddressBus {
    pub buffer: [u8; MEM_MAP_SIZE],
    dummy: u8,
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: [0; MEM_MAP_SIZE],
            dummy: 0,
        }
    }
}

impl Index<u16> for AddressBus {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        &self.buffer[index as usize]
    }
}

impl IndexMut<u16> for AddressBus {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        if (0x0000..0x8000).contains(&index) {
            return &mut self.dummy
        }
        &mut self.buffer[index as usize]
    }
}

impl Index<Range<u16>> for AddressBus {
    type Output = [u8];

    fn index(&self, index: Range<u16>) -> &Self::Output {
        &self.buffer[index.start as usize..index.end as usize]
    }
}

impl IndexMut<Range<u16>> for AddressBus {
    fn index_mut(&mut self, index: Range<u16>) -> &mut Self::Output {
        &mut self.buffer[index.start as usize..index.end as usize]
    }
}