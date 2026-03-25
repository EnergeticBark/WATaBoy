use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;

use hw_constants::{OAM_END, OAM_START, VRAM_END, VRAM_START};

#[derive(Archive, Default, Deserialize, Serialize)]
pub struct WokeCounter(pub HashMap<u16, u64>);

impl WokeCounter {
    pub fn ppu_access(&mut self, index: u16) {
        // Collapse VRAM and OAM read indexes into the start of VRAM or OAM.
        let key = match index {
            VRAM_START..VRAM_END => VRAM_START,
            OAM_START..OAM_END => OAM_START,
            _ => index,
        };
        if let Some(woke_count) = self.0.get_mut(&key) {
            *woke_count += 1;
        } else {
            self.0.insert(key, 1);
        }
    }
}
