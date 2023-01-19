use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

use common::{messages::{FileListingFragment, Message}, MessageReceiver};

use crate::{server_state::ChunkState, comms::ServerCommunicator, progress_indicator::ProgressEvent};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


pub async fn download_file(
    mut listener: MessageReceiver,
    comm: ServerCommunicator,
    file: FileListingFragment,
    mut chunks: ChunkState,
    progress_sender: Sender<ProgressEvent>,
    request_interval_us: u64,
) {
    let mut timeout = if request_interval_us == 0 {
        tokio::time::interval(std::time::Duration::from_secs(10)) // This interval will not be used
    } else {
        tokio::time::interval(std::time::Duration::from_micros(request_interval_us))
    };
    let path = PathBuf::from(&file.path);

    // Listen for messages containing chunks, and if the interval ticks, request the next chunk
    loop {
        tokio::select! {
            _ = timeout.tick() => {
                if request_interval_us == 0 {
                    // This value means that we never request chunks
                    continue;
                }

                if let Some(next_chunk) = chunks.get_zero() {
                    debug!("Requesting chunk {} for file {:?}", next_chunk, file);
                    comm.send_message(&Message::FileChunkRequest{idx: file.idx, chunk: next_chunk}).await;
                    progress_sender.send(ProgressEvent::ChunkRequested(file.idx.into(), next_chunk)).await.expect("Failed to send progress event");
                } else {
                    debug!("All chunks received for file {:?}!", file);
                    progress_sender.send(ProgressEvent::FileDone(file.idx.into())).await.expect("Failed to send progress event");
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
                    progress_sender.send(ProgressEvent::ChunkDownloaded(file.idx.into(), chunk.chunk, chunk.data.len())).await.expect("Failed to send progress event");

                    // If we have all the chunks, we can stop listening
                    if chunks.is_complete() {
                        debug!("All chunks received for file {:?}!", file);
                        progress_sender.send(ProgressEvent::FileDone(file.idx.into())).await.expect("Failed to send progress event");
                        // We need to drain the channel, otherwise it will be dropped and this will stop the download
                        common::channels::drain(listener);
                        break;
                    }
                }
            }
        }
    }
}