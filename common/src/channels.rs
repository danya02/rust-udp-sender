use std::fmt::Debug;
use tokio::sync::mpsc::Receiver;

/// Tools for manipulating channels.
///
/// A channel is a stream of Messages.

/// Consume a channel.
/// Return two others: the first one receives messages that match the predicate, the second one receives the rest.
///
/// If `and_also_other` is true, the second channel will also receive messages that match the predicate.
/// If not, it will only receive messages that don't match the predicate.
/// Be aware that this requires cloning the incoming messages.
pub fn filter_branch_pred<T: Debug + Clone + Send + Sync + 'static>(
    mut from: Receiver<T>,
    predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    and_also_other: bool,
) -> (Receiver<T>, Receiver<T>) {
    let (from1, to1) = tokio::sync::mpsc::channel(10);
    let (from2, to2) = tokio::sync::mpsc::channel(10);
    tokio::spawn(async move {
        while let Some(msg) = from.recv().await {
            if predicate(&msg) {
                if and_also_other {
                    from1.send(msg.clone()).await.unwrap();
                    from2.send(msg).await.unwrap();
                } else {
                    from1.send(msg).await.unwrap();
                }
            } else {
                from2.send(msg).await.unwrap();
            }
        }
    });
    (to1, to2)
}

/// Consume a channel, receive messages in a loop, and discard them.
///
/// Use this at the end of a pipeline of channel transformations to avoid blocking.
pub fn drain<T: Debug + Clone + Send + Sync + 'static>(mut from: Receiver<T>) {
    tokio::spawn(async move {
        while (from.recv().await).is_some() {
            // Discard
        }
    });
}

/// Consume a channel, receive messages in a loop, and discard them, printing them to stdout.
pub fn drain_with_print<T: Debug + Clone + Send + Sync + 'static>(mut from: Receiver<T>) {
    tokio::spawn(async move {
        while let Some(msg) = from.recv().await {
            println!("Discarding: {:?}", msg);
        }
    });
}

/// Consume a channel. Return a new channel, to which the messages are forwarded.
/// Messages are also printed to stdout.
pub fn print<T: Debug + Send + Sync + 'static>(mut recv_from: Receiver<T>) -> Receiver<T> {
    let (from, to) = tokio::sync::mpsc::channel(100);
    tokio::spawn(async move {
        while let Some(msg) = recv_from.recv().await {
            println!("Received: {:?}", msg);
            from.send(msg).await.unwrap();
        }
    });
    to
}
