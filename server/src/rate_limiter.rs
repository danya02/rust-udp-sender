use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use tokio::time::Instant;

pub struct RateLimiter {
    current_packets_per_second: tokio::sync::watch::Receiver<usize>,

    last_accounting_period: Instant,
    packets_this_period: usize,

    ping_stat_sender: tokio::sync::mpsc::Sender<(String, u64)>,

    ping_stats: Arc<Mutex<std::collections::HashMap<String, usize>>>,
}

const PACKETS_DELIVERED_TARGET: f64 = 0.5;

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// Accepts a minimum and maximum number of packets per second.
    /// Panics if the minimum is greater than the maximum.
    ///
    /// The rate limiter's current rate will be set to the minimum.
    pub fn new(min_packets_per_second: usize, max_packets_per_second: usize) -> Self {
        assert!(min_packets_per_second <= max_packets_per_second);
        let ping_stats = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let (sender, receiver) = tokio::sync::mpsc::channel(100);
        let (rate_limit_sender, rate_limit_receiver) =
            tokio::sync::watch::channel(min_packets_per_second);
        let ret = Self {
            current_packets_per_second: rate_limit_receiver,

            last_accounting_period: Instant::now(),
            packets_this_period: 0,

            ping_stat_sender: sender,

            ping_stats: ping_stats.clone(),
        };

        tokio::spawn(rate_limit_thread(
            receiver,
            min_packets_per_second,
            max_packets_per_second,
            rate_limit_sender,
            ping_stats,
        ));
        ret
    }

    /// Get the ping stat collector.
    pub fn get_collector(&self) -> tokio::sync::mpsc::Sender<(String, u64)> {
        self.ping_stat_sender.clone()
    }

    /// Account for a packet being sent.
    ///
    /// If this packet would exceed the rate limit, this function will delay
    /// until the end of the current accounting period.
    pub async fn on_packet(&mut self) {
        let mut ping_stats = self.ping_stats.lock().await;

        self.packets_this_period += 1;

        // Check if the current accounting period has ended
        let accounting_period = Duration::from_secs(1);
        if Instant::now() - self.last_accounting_period > accounting_period {
            self.packets_this_period = 1;
            self.last_accounting_period = Instant::now();
            return;
        }

        // If we've sent too many packets, wait until the end of the accounting period
        if self.packets_this_period > *self.current_packets_per_second.borrow() {
            let time_left = self.last_accounting_period + accounting_period - Instant::now();
            log::info!(
                "Ratelimiting for {time_left:?} after sending {}",
                self.packets_this_period
            );
            tokio::time::sleep(time_left).await;
            self.packets_this_period = 1;
            self.last_accounting_period = Instant::now();
        }

        // Record that every peer should have received a packet
        for (_peer, count) in ping_stats.iter_mut() {
            *count += 1;
        }

        // Delete any peers that haven't sent a ping in a while
        let mut to_delete = Vec::new();
        for (peer, count) in ping_stats.iter() {
            if *count > (*self.current_packets_per_second.borrow() * 100) {
                // Peer that hasn't sent a ping in 100 seconds is probably gone
                to_delete.push(peer.clone());
            }
        }

        for peer in to_delete {
            ping_stats.remove(&peer);
        }
    }
}

/// Thread that manages the rate limit.
///
/// Keeps track of the number of packets sent since a client's ping:
/// this is the number we expect to see in their next ping.
/// If we sent more than that, then some packets must have been dropped,
/// so we decrease the number of packets per second.
///
/// Uses an additive increase, multiplicative decrease algorithm.
///
/// Takes a mpsc::Receiver<(String, u64)>: the number of packets seen by a peer.
/// Takes 2 usize: the minimum and maximum number of packets per second.
/// Takes a watch::Sender<usize>: the current rate limit.
async fn rate_limit_thread(
    mut ping_stat_recv: tokio::sync::mpsc::Receiver<(String, u64)>,
    min_packets_per_second: usize,
    max_packets_per_second: usize,
    current_packets_per_second: tokio::sync::watch::Sender<usize>,
    ping_stats: Arc<Mutex<std::collections::HashMap<String, usize>>>,
) {
    let mut current_pps = min_packets_per_second;
    loop {
        // Receive ping stats
        tokio::select! {
            Some((peer, count)) = ping_stat_recv.recv() => {
                let mut ping_stats = ping_stats.lock().await;
                // If this is the first time we've seen this peer,
                // then we don't know how many packets they've seen yet.
                // So we just set the count to 0.
                if !ping_stats.contains_key(&peer) {
                    ping_stats.insert(peer, 0);
                    continue;
                }

                // If the peer has already been seen, then we can compare
                // the number of packets they've seen to the number we've sent.
                let packets_sent = ping_stats.get(&peer).unwrap();
                let packets_seen = count;
                let packets_delivered_fraction = packets_seen as f64 / *packets_sent as f64;
                if packets_delivered_fraction < PACKETS_DELIVERED_TARGET {
                    // If too many packets were dropped, decrease the rate limit
                    current_pps = (current_pps as f64 * 0.9) as usize;
                    if current_pps < min_packets_per_second {
                        current_pps = min_packets_per_second;
                    }
                    log::info!("Decreasing rate limit to {current_pps} because delivery fraction is {packets_delivered_fraction}");
                    current_packets_per_second.send(current_pps).expect("Rate limit channel closed");
                } else {
                    // If too few packets were dropped, increase the rate limit
                    current_pps = (current_pps + 5).min(max_packets_per_second);
                    log::info!("Increasing rate limit to {current_pps} because delivery fraction is {packets_delivered_fraction}");
                    current_packets_per_second.send(current_pps).expect("Rate limit channel closed");
                }

                // When the ping is received, both the client and the server
                // should reset their packet counts to 0.
                //ping_stats.insert(peer, 0);

            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}
