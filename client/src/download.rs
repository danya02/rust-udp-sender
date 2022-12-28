use std::path::PathBuf;

use common::{messages::{FileListingFragment, Message}, MessageReceiver};

use crate::{server_state::ChunkState, comms::ServerCommunicator};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


pub async fn download_file(
    mut listener: MessageReceiver,
    comm: ServerCommunicator,
    file: FileListingFragment,
    mut chunks: ChunkState
) {
    let mut timeout = tokio::time::interval(std::time::Duration::from_millis(500));
    let path = PathBuf::from(&file.path);

    // Listen for messages containing chunks, and if the interval ticks, request the next chunk
    loop {
        tokio::select! {
            _ = timeout.tick() => {
                if let Some(next_chunk) = chunks.get_zero() {
                    debug!("Requesting chunk {} for file {:?}", next_chunk, file);
                    comm.send_message(&Message::FileChunkRequest{idx: file.idx, chunk: next_chunk}).await;
                } else {
                    debug!("All chunks received for file {:?}!", file);
                    // We need to drain the channel, otherwise it will be dropped and this will stop the download
                    common::channels::drain(listener);
                    break;
                }
            }
            Some((_, _, message)) = listener.recv() => {
                if let Message::FileChunk(chunk) = message {
                    debug!("Got chunk {}-{}", chunk.idx, chunk.chunk);
                    common::filesystem::write_chunk(&path, file.chunk_size as u64, chunk.chunk, &chunk.data).await.expect("Failed to write chunk");
                    chunks.set(chunk.chunk, true);
                }
            }
        }
    }
}