use std::{
    convert::TryInto,
    hash::{BuildHasher, Hasher},
    num::Wrapping,
};
#[derive(Default)]
pub struct BKDRHash {
    hash: u64,
}
impl BKDRHash {
    pub fn new() -> BKDRHash {
        BKDRHash { hash: 0 }
    }
}
impl Hasher for BKDRHash {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for c in bytes {
            self.write_u8(*c);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as u64);
    }

    fn write_u16(&mut self, i: u16) {
        self.write_u64(i as u64);
    }

    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }

    fn write_u64(&mut self, i: u64) {
        self.hash = (Wrapping(i) + Wrapping(31) * Wrapping(self.hash)).0;
    }

    fn write_u128(&mut self, i: u128) {
        let bytes: [u8; 16] = i.to_le_bytes();
        let low: [u8; 8] = bytes[0..8].try_into().unwrap();
        let high: [u8; 8] = bytes[8..16].try_into().unwrap();
        self.write_u64(u64::from_le_bytes(low));
        self.write_u64(u64::from_le_bytes(high));
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}

#[derive(Hash, Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct BuildBKDRHash {}
impl BuildHasher for BuildBKDRHash {
    type Hasher = BKDRHash;

    fn build_hasher(&self) -> Self::Hasher {
        BKDRHash { hash: 0 }
    }
}
