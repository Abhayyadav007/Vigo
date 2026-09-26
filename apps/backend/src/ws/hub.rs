//! One Redis Pub/Sub connection per backend instance, fanned out to any
//! number of local subscribers (WebSocket sessions) through tokio broadcast
//! channels. Channels are subscribed in Redis while at least one local
//! subscriber wants them, and resubscribed after a reconnect.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use deadpool_redis::redis::{self, aio::PubSubSink};
use futures_util::StreamExt;
use tokio::sync::broadcast;

/// Messages buffered per channel before a slow subscriber lags.
const BUFFER: usize = 64;

#[derive(Default)]
struct Inner {
    channels: HashMap<String, broadcast::Sender<Arc<str>>>,
    sink: Option<PubSubSink>,
}

#[derive(Clone)]
pub struct PubSubHub {
    client: redis::Client,
    inner: Arc<Mutex<Inner>>,
}

impl PubSubHub {
    /// Creates the hub and starts its connection task on the current runtime.
    pub fn start(redis_url: &str) -> anyhow::Result<Self> {
        let hub = Self {
            client: redis::Client::open(redis_url)?,
            inner: Arc::default(),
        };
        tokio::spawn(hub.clone().run());
        Ok(hub)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding this lock can't leave the map inconsistent
        // (single inserts/removes), so recover from poisoning.
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Receives every message published to `channel` from now on.
    pub async fn subscribe(&self, channel: &str) -> Subscription {
        let (rx, sink) = {
            let mut inner = self.lock();
            match inner.channels.get(channel) {
                Some(tx) => (tx.subscribe(), None),
                None => {
                    let (tx, rx) = broadcast::channel(BUFFER);
                    inner.channels.insert(channel.to_owned(), tx);
                    (rx, inner.sink.clone())
                }
            }
        };
        if let Some(mut sink) = sink
            && let Err(error) = sink.subscribe(channel).await
        {
            // The connection task resubscribes everything when it reconnects.
            tracing::warn!(%error, channel, "Redis SUBSCRIBE failed");
        }
        Subscription {
            rx: Some(rx),
            hub: self.clone(),
            channel: channel.to_owned(),
        }
    }

    fn release(&self, channel: &str) {
        let sink = {
            let mut inner = self.lock();
            let unused = inner
                .channels
                .get(channel)
                .is_some_and(|tx| tx.receiver_count() == 0);
            if !unused {
                return;
            }
            inner.channels.remove(channel);
            inner.sink.clone()
        };
        if let Some(mut sink) = sink {
            let channel = channel.to_owned();
            tokio::spawn(async move {
                let _ = sink.unsubscribe(&channel).await;
            });
        }
    }

    async fn run(self) {
        let mut backoff = Duration::from_millis(250);
        loop {
            match self.client.get_async_pubsub().await {
                Ok(pubsub) => {
                    backoff = Duration::from_millis(250);
                    let (mut sink, mut stream) = pubsub.split();
                    let channels: Vec<String> = {
                        let mut inner = self.lock();
                        inner.sink = Some(sink.clone());
                        inner.channels.keys().cloned().collect()
                    };
                    if !channels.is_empty()
                        && let Err(error) = sink.subscribe(&channels).await
                    {
                        tracing::warn!(%error, "Redis resubscribe failed");
                    }
                    tracing::debug!(count = channels.len(), "pub/sub hub connected");

                    while let Some(msg) = stream.next().await {
                        let Ok(payload) = msg.get_payload::<String>() else {
                            continue;
                        };
                        let tx = self.lock().channels.get(msg.get_channel_name()).cloned();
                        if let Some(tx) = tx {
                            // No receivers is fine: they just left.
                            let _ = tx.send(Arc::from(payload));
                        }
                    }
                    self.lock().sink = None;
                    tracing::warn!("pub/sub hub lost its Redis connection; reconnecting");
                }
                Err(error) => tracing::warn!(%error, "pub/sub hub can't connect to Redis"),
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(10));
        }
    }
}

/// A live subscription; unsubscribes from Redis when the last one for a
/// channel is dropped.
pub struct Subscription {
    rx: Option<broadcast::Receiver<Arc<str>>>,
    hub: PubSubHub,
    channel: String,
}

impl Subscription {
    pub async fn recv(&mut self) -> Result<Arc<str>, broadcast::error::RecvError> {
        match self.rx.as_mut() {
            Some(rx) => rx.recv().await,
            None => Err(broadcast::error::RecvError::Closed),
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        drop(self.rx.take());
        self.hub.release(&self.channel);
    }
}
