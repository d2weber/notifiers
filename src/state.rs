use std::path::Path;

use a2::{Client, Endpoint};
use anyhow::{Context as _, Result};
use async_std::sync::Arc;
use log::*;
use std::io::Seek;

use crate::metrics::Metrics;

#[derive(Debug, Clone)]
pub struct State {
    inner: Arc<InnerState>,
}

#[derive(Debug)]
pub struct InnerState {
    db: sled::Db,

    production_client: Client,

    sandbox_client: Client,

    topic: Option<String>,

    metrics: Arc<Metrics>,
}

impl State {
    pub fn new(
        db: &Path,
        mut certificate: std::fs::File,
        password: &str,
        topic: Option<String>,
        metrics: Arc<Metrics>,
    ) -> Result<Self> {
        let db = sled::open(db)?;
        let production_client =
            Client::certificate(&mut certificate, password, Endpoint::Production)
                .context("Failed to create production client")?;
        certificate.rewind()?;
        let sandbox_client = Client::certificate(&mut certificate, password, Endpoint::Sandbox)
            .context("Failed to create sandbox client")?;

        info!("{} devices registered currently", db.len());

        Ok(State {
            inner: Arc::new(InnerState {
                db,
                production_client,
                sandbox_client,
                topic,
                metrics,
            }),
        })
    }

    pub fn db(&self) -> &sled::Db {
        &self.inner.db
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
}
