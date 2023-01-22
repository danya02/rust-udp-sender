use common::messages::FileListingFragment;

/// Data structures representing synched state between the server and the client

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

    /// The number of chunks in the file.
    /// This is smaller than the length of the bitmap.
    pub num_chunks: u64,
}

impl ChunkState {
    pub fn from_file_size(size: u64, chunk_size: u16) -> Self {
        let num_chunks = (size + (chunk_size as u64 - 1)) / chunk_size as u64;
        let num_u64s = (num_chunks + 63) / 64;
        Self {
            bitmap: vec![0; num_u64s as usize],
            num_chunks,
        }
    }

    #[allow(dead_code)]
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
            if u64 != !0 {
                // if not all bits are set
                for j in 0..64 {
                    // find the first bit that is not set
                    let mask = 1 << j;
                    if u64 & mask == 0 {
                        let idx = (i as u64) * 64 + j;
                        if idx >= self.num_chunks {
                            // if the index is out of bounds
                            return None;
                        }
                        return Some((i as u64) * 64 + j);
                    }
                }
            }
        }
        None
    }

    /// Check if all chunks are downloaded.
    pub fn is_complete(&self) -> bool {
        self.get_zero().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_chunk_state() {
        let mut state = ChunkState::from_file_size(100, 10);
        state.set(0, true);
        assert!(state.get(0));
        assert!(!state.get(1));
        state.set(1, true);
        assert!(state.get(1));
    }

    #[test]
    fn test_get_zero() {
        let mut state = ChunkState::from_file_size(100, 10);
        assert_eq!(state.get_zero(), Some(0));
        state.set(0, true);
        assert_eq!(state.get_zero(), Some(1));
        state.set(1, true);
        assert_eq!(state.get_zero(), Some(2));
        state.set(2, true);
        assert_eq!(state.get_zero(), Some(3));
        state.set(3, true);
        assert_eq!(state.get_zero(), Some(4));
        state.set(4, true);
        assert_eq!(state.get_zero(), Some(5));
        state.set(5, true);
        assert_eq!(state.get_zero(), Some(6));
        state.set(6, true);
        assert_eq!(state.get_zero(), Some(7));
        state.set(7, true);
        assert_eq!(state.get_zero(), Some(8));
        state.set(8, true);
        assert_eq!(state.get_zero(), Some(9));
        state.set(9, true);
        assert_eq!(state.get_zero(), None);
    }
}
