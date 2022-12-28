use common::{MessageReceiver, messages::Message};

use crate::server_state::ServerData;
/// Split a message receiver into many.
/// Each of them will only receive messages corresponding to one of the given files.
/// An extra receiver is returned, which will receive all messages that don't correspond to any of the given files.
pub fn split_by_files(mut listener: MessageReceiver, data: ServerData) -> (Vec<MessageReceiver>, MessageReceiver) {
    let mut senders = vec![];
    let mut receivers = vec![];
    for file in data.files.iter() {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);
        senders.push(sender);
        receivers.push(receiver);
    }
    let (extra_sender, extra_receiver) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        while let Some((src, name, message)) = listener.recv().await {
            match &message {
                Message::FileChunk(chunk) => {
                    let send_to = {
                        let mut maybe_sender = None;
                        for (i, _file) in data.files.iter().enumerate() {
                            if (i as u32) == chunk.idx {
                                maybe_sender = Some(&senders[i]);
                            }
                        }
                        if let Some(sender) = maybe_sender {
                            sender
                        } else {
                            &extra_sender
                        }
                    };
                    send_to.send((src, name, message)).await.unwrap();
                },
                _ => {
                    extra_sender.send((src, name, message)).await.unwrap();
                },
            }
        }
    });

    (receivers, extra_receiver)
}