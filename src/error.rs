// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MertonError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTML parsing error: {0}")]
    Parse(String),

    #[error("Property with ID '{0}' was not found")]
    PropertyNotFound(String),

    #[error("Timed out waiting for bin collection data to load for property '{0}'")]
    Timeout(String),

    #[error("Invalid property ID or URL: {0}")]
    InvalidInput(String),
}
