use std::io::Seek;
use std::path::Path;
use std::time::Duration;

use a2::{Client, Endpoint};
use anyhow::{Context as _, Result};
use async_std::sync::Arc;

use crate::metrics::Metrics;
use crate::schedule::Schedule;

#[derive(Debug, Clone)]
pub struct State {
    inner: Arc<InnerState>,
}

#[derive(Debug)]
pub struct InnerState {
    schedule: Schedule,

    production_client: Client,

    sandbox_client: Client,

    topic: Option<String>,

    metrics: Arc<Metrics>,

    /// Heartbeat notification interval.
    interval: Duration,
}

impl State {
    pub fn new(
        db: &Path,
        mut certificate: std::fs::File,
        password: &str,
        topic: Option<String>,
        metrics: Arc<Metrics>,
        interval: Duration,
    ) -> Result<Self> {
        let schedule = Schedule::new(db)?;
        let production_client =
            Client::certificate(&mut certificate, password, Endpoint::Production)
                .context("Failed to create production client")?;
        certificate.rewind()?;
        let sandbox_client = Client::certificate(&mut certificate, password, Endpoint::Sandbox)
            .context("Failed to create sandbox client")?;

        Ok(State {
            inner: Arc::new(InnerState {
                schedule,
                production_client,
                sandbox_client,
                topic,
                metrics,
                interval,
            }),
        })
    }

    pub fn schedule(&self) -> &Schedule {
        &self.inner.schedule
    }

    pub fn production_client(&self) -> &Client {
        &self.inner.production_client
    }

    pub fn sandbox_client(&self) -> &Client {
        &self.inner.sandbox_client
    }

    pub fn topic(&self) -> Option<&str> {
        self.inner.topic.as_deref()
    }

    pub fn metrics(&self) -> &Metrics {
        self.inner.metrics.as_ref()
    }

    pub fn interval(&self) -> Duration {
        self.inner.interval
    }
}
