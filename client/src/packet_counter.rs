use tokio::sync::{mpsc, watch};

/// Thread to maintain a count of packets that pass through the pipeline.
///
/// # Arguments
/// listener: an `mpsc::Receiver` that receives packets
/// sender: an `mpsc::Sender` to forward packets to
/// count_sender: an `watch::Sender` to send the count to
/// reset_receiver: a `watch::Receiver<()>`. When its `changed()` future fires, the count is reset to 0.
pub async fn count_packets<T: std::fmt::Debug>(
    mut listener: mpsc::Receiver<T>,
    sender: mpsc::Sender<T>,
    count_sender: watch::Sender<u64>,
    mut reset_receiver: watch::Receiver<()>,
) {
    let mut count = 0;
    loop {
        tokio::select! {
            Some(packet) = listener.recv() => {
                count += 1;
                count_sender.send(count).unwrap();
                sender.send(packet).await.unwrap();
            }
            _ = reset_receiver.changed() => {
                //count = 0;
                count_sender.send(count).unwrap();
            }
        }
    }
}
