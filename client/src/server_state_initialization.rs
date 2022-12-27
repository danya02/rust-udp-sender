use common::{MessageReceiver, messages};

use crate::comms::ServerCommunicator;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


/// Function to talk to the server to initialize the state.
pub async fn initialize_state(listener: &mut MessageReceiver, comm: ServerCommunicator) -> crate::server_state::ServerData {
    let mut timeout = tokio::time::interval(std::time::Duration::from_millis(500));
    // First, we need to figure out how many files there are
    // If we see a FileListing, we can figure it out, otherwise we need to request it
    let num_files;
    let mut attempts = 0;
    loop {
        tokio::select! {
            _ = timeout.tick() => {
                debug!("Requesting file listing of file 0 to get number of files");
                attempts += 1;
                if attempts > 10 {
                    error!("Failed to get file listing");
                    std::process::exit(1);
                }
                comm.send_message(&messages::Message::FileListingRequest(0)).await;
            }
            Some((_, _, message)) = listener.recv() => {
                if let messages::Message::FileListing(listing_item) = message {
                    num_files = listing_item.total;
                    debug!("Got file listing, there are {} files", num_files);
                    break;
                }
            }
        }
    }

    // Now we need to get all the file listings
    let mut file_listings = vec![None; num_files as usize];
    loop { 
        tokio::select! {
            // If we don't get any file listings for a while, request them again
            _ = timeout.tick() => {
                let mut requested = 0;
                for (i, listing) in file_listings.iter().enumerate() {
                    if listing.is_none() {
                        debug!("Requesting file listing of file {}", i);
                        comm.send_message(&messages::Message::FileListingRequest(i as u32)).await;
                        requested += 1;
                        if requested > 50 {
                            debug!("Waiting for some responses before continuing");
                            break;
                        }
                    }
                }
            }
            Some((_, _, message)) = listener.recv() => {
                if let messages::Message::FileListing(listing_item) = message {
                    let idx = listing_item.idx as usize;
                    file_listings[idx] = Some(listing_item);
                    debug!("Got file listing for file {}", idx);
                }
            }
        }
        if file_listings.iter().all(|x| x.is_some()) {
            debug!("Got all file listings!");
            break;
        }
    }

    // Now create and return the state
    let state = crate::server_state::ServerData {
        files: file_listings.into_iter().map(|x| {
            // The first item is the file listing
            // The second item is the ChunkState
            let listing = x.unwrap();
            let chunk_data = crate::server_state::ChunkState::from_file_size(listing.size, listing.chunk_size);
            (listing, chunk_data)
        }).collect(),
    };
    
    state

}