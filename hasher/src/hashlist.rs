use serde::{Deserialize, Serialize};

/// Module containing the HashList structure,
/// which is used to store a list of hashes for a directory.

/// A HashList is a structure that stores files, their lengths, and their hashes.
/// This is published by the server to the network, and is used by clients to
/// determine which files they need to download.
#[derive(Serialize, Deserialize, Debug)]
pub struct HashList {
    /// The name of the hash algorthm used to hash the files.
    /// This is currently hardcoded to "sha256",
    /// and users should compare this to their supported hash algorithms.
    pub hash_algorithm: String,

    /// The list of files in the directory.
    pub files: Vec<FileHashItem>,
}

/// A FileHashItem is a structure that stores a file's name, length, and hash.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FileHashItem {
    /// The relative path of the file.
    pub path: String,

    /// The size of the file in bytes.
    pub size: u64,

    /// The hash of the file.
    /// The kind of hash is specified by the hash_algorithm field in the HashList.
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
}

impl FileHashItem {
    /// Create a FileHashItem representing a non-existent file.
    /// It has a size of 0, and a hash of all zeros
    /// (the real hash of an empty file is different).
    pub fn nonexistent(path: &str) -> Self {
        Self {
            path: path.to_string(),
            size: 0,
            hash: vec![0; 32],
        }
    }

    /// Create a FileHashItem representing a non-existent file with the empty path.
    pub fn nonexistent_empty_path() -> Self {
        Self::nonexistent("")
    }
}