/// Benchmark for the performance of channel communication.
use tokio::{select, sync::mpsc::*};

#[derive(Debug, Clone)]
struct Message {
    data: [u8; 1024],
}

async fn message_generator(sender: Sender<Message>, max_ind: u8) {
    loop {
        for ind in 0..max_ind {
            sender.send(Message { data: [ind; 1024] }).await.unwrap();
        }
    }
}

async fn message_consumer(mut receiver: Receiver<Message>, my_id: u8) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));
    let mut messages = 0;
    let mut intervals = 0;
    loop {
        select! {
            Some(_message) = receiver.recv() => {
                messages += 1;
            },
            _ = interval.tick() => {
                intervals += 1;
                println!("{}: {} m/int", my_id, messages/intervals);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = channel(10000);
    let destinations = 1;
    let mut rest_of_receivers = receiver;
    for i in 0..destinations {
        let (this_receiver, other_receivers) = common::channels::filter_branch_pred(
            rest_of_receivers,
            move |msg: &Message| msg.data[0] == i,
            false,
        );
        tokio::spawn(message_consumer(this_receiver, i));
        rest_of_receivers = other_receivers;
    }
    tokio::spawn(message_generator(sender, destinations));
    common::channels::drain(rest_of_receivers);
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
