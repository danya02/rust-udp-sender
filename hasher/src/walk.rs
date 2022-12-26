use async_recursion::async_recursion;
/// Module for walking a directory and hashing its contents.

use sha2::Digest;
use std::path::{PathBuf, Path};

use tokio::{sync::mpsc::Sender, task::JoinHandle, io::AsyncReadExt};

use crate::hashlist::{HashList, FileHashItem};


#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};



/// Create a thread that listens for messages containing directory entries,
/// stores them, and returns when the list is complete.
pub(crate) fn collect_entries() -> (Sender<FileHashItem>, JoinHandle<HashList>) {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);
    let handle = tokio::spawn(async move {
        let mut files = Vec::new();
        while let Some(file) = receiver.recv().await {
            files.push(file);
        }
        HashList {
            hash_algorithm: "sha256".to_string(),
            files,
        }
    });
    (sender, handle)
}

/// Get the Sha256 hash of a file.
pub(crate) async fn get_file_hash(path: impl AsRef<Path>) -> [u8; 32] {
    let mut file = tokio::fs::File::open(path).await.expect("Unable to open file");
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0; 1024];

    loop {
        let n = file.read(&mut buf).await.expect("Unable to read file");
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let mut hash = [0; 32];
    hash.copy_from_slice(hasher.finalize().as_slice());
    hash
}


/// Walk a directory.
/// For subdirectories, spawn a new thread to walk them.
/// For files, hash them and send the result to the main thread.
/// 
/// For the initial invocation, both the `path` and the `base` should be the same.
#[async_recursion]
pub(crate) async fn walk_directory_and_hash(
    path: PathBuf,
    base: PathBuf,
    sender: Sender<FileHashItem>,
) {
    let mut pending_inner_tasks = vec![];
    let mut dir_listing = tokio::fs::read_dir(&path).await.expect("Failed to read directory");

    while let Ok(entry) = dir_listing.next_entry().await {
        if let Some(entry) = entry {
            let path = entry.path();

            // If directory, recurse
            if path.is_dir() {
                debug!("Found directory: {:?}", path);
                let sender_copy = sender.clone();
                let base_copy = base.clone();
                let handle = tokio::spawn(walk_directory_and_hash(path, base_copy, sender_copy));
                pending_inner_tasks.push(handle);

            } else {
                debug!("Found file: {:?}", path);
                let file_size = tokio::fs::metadata(&path).await.unwrap().len();
                let item = FileHashItem {
                    path: path.strip_prefix(base.clone()).unwrap().to_str().unwrap().to_string(),
                    size: file_size,
                    hash: get_file_hash(&path).await.to_vec(),
                };
                sender.send(item).await.unwrap();
            }
        } else {
            break;
        }
    }

    // Wait for all inner tasks to finish
    for task in pending_inner_tasks {
        task.await.unwrap();
    }
}
