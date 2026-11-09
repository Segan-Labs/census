use std::time::Duration;
use serde_json::Value;
use anyhow::Result;

pub trait Collector: Send + Sync {
    fn id(&self) -> &'static str;

    fn interval(&self) -> Duration;

    fn collect(&self) -> impl std::future::Future<Output = Result<Value>> + Send;
}
