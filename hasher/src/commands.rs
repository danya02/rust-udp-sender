use std::path::PathBuf;

use crate::{walk, args::{VerifyOptions, HashOptions}, hashlist::FileHashItem};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};



pub(crate) async fn make_hash(options: HashOptions) {
    let path = match options.path {
        Some(path) => path,
        None => ".".to_string(),
    };
    let path = PathBuf::from(path).canonicalize().expect("Invalid path to directory to hash");
    let file = options.file;
    let file = PathBuf::from(file);


    println!("Hashing directory: {path:?}");
    println!("Will write hashlist to: {file:?}");

    let (sender, handle) = walk::collect_entries();
    let path2 = path.clone();
    walk::walk_directory_and_hash(path, path2, sender).await;
    let hashlist = handle.await.expect("Failed to get hashlist from thread");
    
    println!("Writing hashlist to: {file:?}...");
    let mut file = std::fs::File::create(file).expect("Failed to create hashlist file");
    rmp_serde::encode::write_named(&mut file, &hashlist).expect("Failed to write hashlist file");

}

fn print_discrepancy(expected: &FileHashItem, actual: &FileHashItem) {
    // test/path: <size>-<hexhash> vs <size>-<hexhash>
    println!("{:?}: {}-{} vs {}-{}", expected.path, expected.size, hex::encode(&expected.hash), actual.size, hex::encode(&actual.hash));

}

pub(crate) async fn verify_hash(options: VerifyOptions) {
    let path = match options.path {
        Some(path) => path,
        None => ".".to_string(),
    };
    let path = PathBuf::from(path).canonicalize().expect("Invalid path to directory to hash");
    let file = options.file;
    let file = PathBuf::from(file).canonicalize().expect("Invalid path to hashlist file (does it exist?)");

    println!("Verifying directory: {path:?}");

    println!("Reading hashlist from: {file:?}...");
    let hashlist: crate::hashlist::HashList = rmp_serde::from_read(std::fs::File::open(file).expect("Failed to open hashlist file")).expect("Failed to parse hashlist file");

    // For every entry in the hashlist, check that the file exists (unless set to ignore missing files),
    // that the length matches, and that the hash matches.
    // Also record every path we've seen: we will use this to check for extra files.
    let mut errors = 0;
    let mut seen_paths = std::collections::HashSet::new();
    debug!("Checking for errors for files in hashlist...");
    for (idx, entry) in hashlist.files.iter().enumerate() {
        if idx % 100 == 0 {
            debug!("Checked {} files out of {}", idx, hashlist.files.len());
        }
        let path = path.join(&entry.path);
        if !path.exists() {
            if options.ignore_missing {
                continue;
            } else {
                print_discrepancy(entry, &FileHashItem::nonexistent_empty_path());
                errors += 1;
                continue;
            }
        }
        let metadata = std::fs::metadata(&path).expect("Failed to get file metadata");
        let hash = walk::get_file_hash(&path).await;
        let actual = FileHashItem {
            path: entry.path.clone(),
            size: metadata.len(),
            hash: hash.to_vec(),
        };
        if entry != &actual {
            print_discrepancy(entry, &actual);
            errors += 1;
        }

        if !options.ignore_new {
            seen_paths.insert(PathBuf::from(&entry.path));
        }
    }

    if !options.ignore_new {
        // Now, walk the directory and check for any files that weren't in the hashlist.
        debug!("Checking for new files...");
        #[async_recursion::async_recursion]
        async fn walk_directory(path: PathBuf, base: PathBuf, seen_paths: &std::collections::HashSet<PathBuf>) -> usize {
            trace!("Walking directory: {:?}", path);
            let mut errors = 0;
            let mut dir_listing = tokio::fs::read_dir(&path).await.expect("Failed to read directory");

            while let Ok(entry) = dir_listing.next_entry().await {
                if let Some(entry) = entry {
                    let path = entry.path();
                    let metadata = entry.metadata().await.expect("Failed to get file metadata");
                    if metadata.is_dir() {
                        errors += walk_directory(path, base.clone(), seen_paths).await;
                    } else {
                        let path = path.strip_prefix(&base).expect("Failed to strip base path from file path");
                        if !seen_paths.contains(path) {
                            debug!("Found new file: {:?}", path);
                            // Get the file's size and hash
                            let hash = walk::get_file_hash(base.join(path)).await;
                            let size = metadata.len();
                            print_discrepancy(&FileHashItem::nonexistent(path.to_str().unwrap()), &FileHashItem {
                                path: path.to_str().unwrap().to_string(),
                                size,
                                hash: hash.to_vec(),
                            });
                            errors += 1;
                        }
                    }
                } else {
                    break;
                }
            }

            errors
        }

        errors += walk_directory(path.clone(), path, &seen_paths).await;
    }

    if errors == 0 {
        println!("No discrepancies found.");
    } else {
        println!("Found {errors} discrepancies.");
    }




}