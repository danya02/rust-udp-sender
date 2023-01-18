use std::{path::PathBuf, str::FromStr};
use common::{MessageReceiver, messages::{FileListingFragment, Message, FileChunkData}};
use hasher::hashlist;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


/// Code that deals with files and file transfers.

/// Convert a hashlist into a vector of FileListingFragments,
/// used for transmitting the file listing.
pub fn hashlist_into_file_listing(hashlist: hashlist::HashList) -> Vec<FileListingFragment> {
    let mut file_listing = Vec::with_capacity(hashlist.files.len());
    let len = hashlist.files.len();
    for (idx, item) in hashlist.files.into_iter().enumerate() {
        let path = PathBuf::from_str(&item.path).unwrap();
        let hash = item.hash.try_into().unwrap();
        let file_listing_fragment = FileListingFragment {
            idx: idx as u32,
            total: len as u32,
            path: path.to_str().unwrap().to_string(),
            hash,
            size: item.size,
            chunk_size: 512,
        };
        file_listing.push(file_listing_fragment);
    }
    file_listing
}

pub async fn run_transmissions(
    transmission_listener: MessageReceiver,
    directory_entries: Vec<FileListingFragment>,
    broadcaster: crate::broadcaster::MessageSender,
    base: PathBuf,
) {
    // Transmit all the directory entries over a period of 5 seconds
    // Also listen for file requests and transmit those out of order
    debug!("Starting file listing transmission thread");
    let (mut file_listing_request_listener, listener) = common::channels::filter_branch_pred(transmission_listener, |msg|{
        matches!(msg.2, common::messages::Message::FileListingRequest { .. })
    }, false);

    let single_entry_duration = std::time::Duration::from_secs(5) / directory_entries.len() as u32;
    let directory_entries_out = directory_entries.clone();
    let broadcaster_out = broadcaster.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(single_entry_duration);
        let mut current_index = 0;
        loop { 
            let message = tokio::select! {
                _ = interval.tick() => {
                    let entry = &directory_entries_out[current_index];
                    let message = common::messages::Message::FileListing( entry.clone() );
                    if current_index == 0 {
                        debug!("Starting to transmit directory entries");
                    }
                    current_index += 1;
                    current_index %= directory_entries_out.len();
                    message
                },
                Some((_, _, message)) = file_listing_request_listener.recv() => {
                    // Got a request for a file listing entry
                    let message = match message {
                        common::messages::Message::FileListingRequest{idx} => {
                            // If the idx is out of bounds, just send the last entry
                            debug!("Got request for file listing entry: {}", idx);
                            let idx = idx.min(directory_entries_out.len() as u32 - 1);
                            let entry = &directory_entries_out[idx as usize];
                            common::messages::Message::FileListing( entry.clone() )
                        },
                        _ => unreachable!(),
                    };
                    message
                },
            };
            debug!("Sending message: {:?}", message);
            broadcaster_out.send(message).await.unwrap();
        }
    });

    // Listen for file requests and transmit those out of order
    debug!("Starting file chunk reply thread");
    let directory_entries_out = directory_entries.clone();
    let broadcaster_out = broadcaster.clone();

    let (mut file_chunk_listener, listener) = common::channels::filter_branch_pred(listener, |msg|{
        matches!(msg.2, common::messages::Message::FileChunkRequest { .. })
    }, false);
    let base_out = base.clone();

    tokio::spawn(async move {
        loop {
            while let Some((_, _, message)) = file_chunk_listener.recv().await {
                match message {
                    Message::FileChunkRequest { idx, chunk: chunk_idx } => {
                        // If the idx is out of bounds, send the last entry
                        if idx > directory_entries_out.len().try_into().unwrap() {
                            broadcaster_out.send(
                                Message::FileListing(directory_entries_out.last().unwrap().clone())
                            ).await.expect("Failed to send file listing entry");
                            continue;
                        }
                        let entry = &directory_entries_out[idx as usize];
                        let chunk_size = entry.chunk_size.into();
                        let chunk_count = (entry.size + chunk_size - 1) / chunk_size;
                        // If the chunk_idx is out of bounds, send the last chunk
                        let chunk_idx = chunk_idx.min(chunk_count - 1);
                        let path = base_out.join(&entry.path);
                        let data_piece = common::filesystem::read_chunk(&path, chunk_size, chunk_idx).await.expect("Failed to read piece of file");
                        let message = Message::FileChunk ( FileChunkData{
                            idx,
                            chunk: chunk_idx,
                            data: data_piece,
                        });
                        broadcaster_out.send(message).await.unwrap();

                    },
                    _ => unreachable!(),
                }
            }
        }
    });

    let directory_entries_out = directory_entries;
    let base_out = base.clone();

    // Also transmit unsolicited file chunks
    tokio::spawn(async move {
        let mut current_file_idx = 0;
        let mut current_chunk_idx = 0;
        loop {
            // get chunk contents
            let entry = &directory_entries_out[current_file_idx];
            let chunk_size = entry.chunk_size.into();
            let chunk_count = (entry.size + chunk_size - 1) / chunk_size;
            let path = base_out.join(&entry.path);
            let data_piece = common::filesystem::read_chunk(&path, chunk_size, current_chunk_idx).await.expect("Failed to read piece of file");
            let message = Message::FileChunk ( FileChunkData{
                idx: current_file_idx as u32,
                chunk: current_chunk_idx,
                data: data_piece,
            });
            // send chunk contents
            broadcaster.send(message).await.unwrap();

            // increment chunk (and file if necessary)
            current_chunk_idx += 1;
            if current_chunk_idx >= chunk_count {
                current_chunk_idx = 0;
                current_file_idx += 1;
                current_file_idx %= directory_entries_out.len();
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // Any other messages are ignored
    common::channels::drain(listener);


}