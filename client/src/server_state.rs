use common::messages::FileListingFragment;

/// Data structures representing synched state between the server and the client


pub struct ServerData {
    /// The files that the server has, as well as our download state.
    /// 
    /// The first element is the file listing fragment, which contains the file name, size and hash
    /// of the file.
    /// The second element is the ChunkState,
    /// which contains information about which chunks of the file are okay.
    pub files: Vec<(FileListingFragment, ChunkState)>,

}

/// The state of the chunks of a file, packed into a bitmap.
pub struct ChunkState {
    /// The bitmap of which chunks have been downloaded.
    /// 
    /// The bits are packed into u64s, so the bitmap is `num_chunks / 64` u64s long.
    /// 
    /// The least significant bit of the first u64 is the first chunk, and so on.
    /// 
    /// If the number of chunks is not a multiple of 64, the last u64 will have
    /// some unused bits at the end.
    bitmap: Vec<u64>,
}

impl ChunkState {
    pub fn from_file_size(size: u64, chunk_size: u16) -> Self {
        let num_chunks = (size + (chunk_size as u64 - 1)) / chunk_size as u64;
        let num_u64s = (num_chunks + 63) / 64;
        Self {
            bitmap: vec![0; num_u64s as usize],
        }
    }

    pub fn get(&self, idx: u64) -> bool {
        let u64_idx = idx / 64;
        let bit_idx = idx % 64;
        let mask = 1 << bit_idx;
        self.bitmap[u64_idx as usize] & mask != 0
    }

    pub fn set(&mut self, idx: u64, val: bool) {
        let u64_idx = idx / 64;
        let bit_idx = idx % 64;
        let mask = 1 << bit_idx;
        if val {
            self.bitmap[u64_idx as usize] |= mask;
        } else {
            self.bitmap[u64_idx as usize] &= !mask;
        }
    }
    /// Find the first chunk that is not downloaded.
    pub fn get_zero(&self) -> Option<u64> {
        for (i, &u64) in self.bitmap.iter().enumerate() {
            if u64 != !0 { // if not all bits are set
                for j in 0..64 { // find the first bit that is not set
                    let mask = 1 << j;
                    if u64 & mask == 0 {
                        return Some((i as u64) * 64 + j);
                    }
                }
            }
        }
        None
    }
}