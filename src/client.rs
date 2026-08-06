// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;

use crate::error::MertonError;
use crate::models::{AddressSearchResult, PropertyBinSchedule};
use crate::parser::{parse_bin_schedule, parse_postcode_addresses};

const BASE_URL: &str = "https://fixmystreet.merton.gov.uk";
const DEFAULT_USER_AGENT: &str = "curl/8.7.1";

/// Client for interacting with the Merton Council bin collection service.
#[derive(Debug, Clone)]
pub struct MertonClient {
    client: Client,
    base_url: String,
    max_poll_attempts: usize,
    poll_interval: Duration,
}

impl Default for MertonClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MertonClient {
    /// Creates a new `MertonClient` with default settings.
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(DEFAULT_USER_AGENT),
        );

        let client = Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: BASE_URL.to_string(),
            max_poll_attempts: 15,
            poll_interval: Duration::from_secs(1),
        }
    }

    /// Sets the maximum number of polling attempts when waiting for bin collection data to render.
    pub fn with_max_poll_attempts(mut self, attempts: usize) -> Self {
        self.max_poll_attempts = attempts;
        self
    }

    /// Sets the poll interval duration between retry requests.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Search for property addresses by postcode.
    ///
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use merton_bin_collection::MertonClient;
    ///
    /// let client = MertonClient::new();
    /// let addresses = client.search_by_postcode("SW19 8ED").await?;
    /// for addr in addresses {
    ///     println!("{}: {}", addr.id, addr.address);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_by_postcode(&self, postcode: &str) -> Result<Vec<AddressSearchResult>, MertonError> {
        let url = format!("{}/waste", self.base_url);
        let params = [("postcode", postcode)];

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MertonError::Parse(format!(
                "Postcode lookup returned HTTP status {}",
                response.status()
            )));
        }

        let body = response.text().await?;
        parse_postcode_addresses(&body)
    }

    /// Retrieves bin collection schedule by property ID or full page URL.
    ///
    /// # Arguments
    /// * `property_id_or_url` - Can be either a property ID (e.g. `"4258300"`)
    ///   or a full URL (e.g. `"https://fixmystreet.merton.gov.uk/waste/4258300"`).
    ///
    /// # Example
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use merton_bin_collection::MertonClient;
    ///
    /// let client = MertonClient::new();
    /// let schedule = client.get_collections("4258300").await?;
    /// println!("Address: {:?}", schedule.address);
    /// for item in schedule.collections {
    ///     println!("{}: next on {:?}", item.service_name, item.next_collection);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_collections(&self, property_id_or_url: &str) -> Result<PropertyBinSchedule, MertonError> {
        let property_id = extract_property_id(property_id_or_url)?;

        let initial_url = format!("{}/waste/{}", self.base_url, property_id);
        let poll_url = format!("{}/waste/{}?page_loading=1", self.base_url, property_id);

        // Initial request to trigger background server loading
        let initial_resp = self.client.get(&initial_url).send().await?;
        if initial_resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(MertonError::PropertyNotFound(property_id));
        }

        // Poll until loading indicator disappears
        for _attempt in 0..self.max_poll_attempts {
            let resp = self
                .client
                .get(&poll_url)
                .header("X-Requested-With", "fetch")
                .send()
                .await?;

            if resp.status() == reqwest::StatusCode::NOT_FOUND {
                return Err(MertonError::PropertyNotFound(property_id.clone()));
            }

            let body = resp.text().await?;

            if !body.contains("loading-indicator") && !body.contains("Loading your bin days...") {
                return parse_bin_schedule(&property_id, &body);
            }

            async_sleep(self.poll_interval).await;
        }

        Err(MertonError::Timeout(property_id))
    }
}

/// Helper function to sleep asynchronously without depending on a specific async runtime.
fn async_sleep(duration: Duration) -> impl std::future::Future<Output = ()> {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll};

    struct SleepFuture {
        done: Arc<AtomicBool>,
        started: bool,
        duration: Duration,
    }

    impl Future for SleepFuture {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.done.load(Ordering::Relaxed) {
                return Poll::Ready(());
            }
            if !self.started {
                self.started = true;
                let done = self.done.clone();
                let waker = cx.waker().clone();
                let duration = self.duration;
                std::thread::spawn(move || {
                    std::thread::sleep(duration);
                    done.store(true, Ordering::Relaxed);
                    waker.wake();
                });
            }
            Poll::Pending
        }
    }

    SleepFuture {
        done: Arc::new(AtomicBool::new(false)),
        started: false,
        duration,
    }
}

/// Helper function to extract property ID from a raw ID or full Merton URL.
pub fn extract_property_id(input: &str) -> Result<String, MertonError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(MertonError::InvalidInput("Input is empty".to_string()));
    }

    if input.starts_with("http://") || input.starts_with("https://") {
        if let Ok(url) = reqwest::Url::parse(input) {
            let segments: Vec<&str> = url
                .path_segments()
                .map(|c| c.collect())
                .unwrap_or_default();

            // Expected path: ["waste", "4258300"]
            if let Some(pos) = segments.iter().position(|&s| s == "waste") {
                if pos + 1 < segments.len() && !segments[pos + 1].is_empty() {
                    return Ok(segments[pos + 1].to_string());
                }
            }
        }
        return Err(MertonError::InvalidInput(format!(
            "Could not extract property ID from URL: {}",
            input
        )));
    }

    // Direct property ID (numeric or alphanumeric string)
    if input.chars().all(|c| c.is_alphanumeric()) {
        Ok(input.to_string())
    } else {
        Err(MertonError::InvalidInput(format!(
            "Invalid property ID format: {}",
            input
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_property_id() {
        assert_eq!(extract_property_id("4258300").unwrap(), "4258300");
        assert_eq!(
            extract_property_id("https://fixmystreet.merton.gov.uk/waste/4258300").unwrap(),
            "4258300"
        );
        assert_eq!(
            extract_property_id("https://fixmystreet.merton.gov.uk/waste/4258300?page_loading=1").unwrap(),
            "4258300"
        );
        assert!(extract_property_id("invalid url/path").is_err());
    }
}
