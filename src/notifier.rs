use a2::{
    Client, DefaultNotificationBuilder, Error::ResponseError, NotificationBuilder,
    NotificationOptions, Priority,
};
use anyhow::Result;
use log::*;

use crate::metrics::Metrics;
use crate::state::State;

pub async fn start(state: State, interval: std::time::Duration) -> Result<()> {
    let db = state.db();
    let metrics = state.metrics();
    let production_client = state.production_client();
    let sandbox_client = state.sandbox_client();
    let topic = state.topic();

    info!(
        "Waking up devices every {}",
        humantime::format_duration(interval)
    );

    loop {
        let wakeup_start = std::time::Instant::now();
        wakeup(db, metrics, production_client, sandbox_client, topic).await;
        let elapsed = wakeup_start.elapsed();
        info!(
            "Waking up all devices took {}",
            humantime::format_duration(elapsed)
        );
        async_std::task::sleep(interval.saturating_sub(elapsed)).await;
    }
}

async fn wakeup(
    db: &sled::Db,
    metrics: &Metrics,
    production_client: &Client,
    sandbox_client: &Client,
    topic: Option<&str>,
) {
    let tokens = db
        .iter()
        .filter_map(|entry| match entry {
            Ok((key, _)) => Some(String::from_utf8(key.to_vec()).unwrap()),
            Err(_) => None,
        })
        .collect::<Vec<_>>();

    info!("sending notifications to {} devices", tokens.len());
    metrics.set_heartbeat_token_count(tokens.len());

    for key_device_token in tokens {
        info!("notify: {}", key_device_token);

        let (client, device_token) =
            if let Some(sandbox_token) = key_device_token.strip_prefix("sandbox:") {
                (sandbox_client, sandbox_token)
            } else {
                (production_client, key_device_token.as_str())
            };

        // Send silent notification.
        // According to <https://developer.apple.com/documentation/usernotifications/generating-a-remote-notification>
        // to send a silent notification you need to set background notification flag `content-available` to 1
        // and don't include `alert`, `badge` or `sound`.
        let payload = DefaultNotificationBuilder::new()
            .set_content_available()
            .build(
                device_token,
                NotificationOptions {
                    // Normal priority (5) means
                    // "send the notification based on power considerations on the user’s device".
                    // <https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns>
                    apns_priority: Some(Priority::Normal),
                    apns_topic: topic,
                    ..Default::default()
                },
            );

        match client.send(payload).await {
            Ok(res) => match res.code {
                200 => {
                    info!("delivered notification for {}", device_token);
                    metrics.inc_heartbeat_notification();
                }
                _ => {
                    warn!("unexpected status: {:?}", res);
                }
            },
            Err(ResponseError(res)) => {
                info!(
                    "Removing token {} due to error {:?}.",
                    &key_device_token, res
                );
                if let Err(err) = db.remove(&key_device_token) {
                    error!("failed to remove {}: {:?}", &key_device_token, err);
                }
            }
            Err(err) => {
                error!(
                    "failed to send notification: {}, {:?}",
                    key_device_token, err
                );
            }
        }
    }
}
