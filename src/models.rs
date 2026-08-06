// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[cfg(feature = "time")]
pub type Date = time::Date;

#[cfg(feature = "time")]
pub type DateTime = time::PrimitiveDateTime;

#[cfg(all(feature = "chrono", not(feature = "time")))]
pub type Date = chrono::NaiveDate;

#[cfg(all(feature = "chrono", not(feature = "time")))]
pub type DateTime = chrono::NaiveDateTime;

/// Represents an address search result returned when looking up a postcode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressSearchResult {
    /// Merton Council property / UPRN ID (e.g. "4258300")
    pub id: String,
    /// Full address string (e.g. "2 Gresham Way, Wimbledon, London, SW19 8ED")
    pub address: String,
}

/// Represents a single bin collection service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// Service name (e.g. "Food waste", "Mixed recycling", "Paper and card", "Non-recyclable waste", "Bulky waste")
    pub service_name: String,
    /// Collection frequency if available (e.g. "Every Friday", "Every other Friday")
    pub frequency: Option<String>,
    /// Next scheduled collection date
    #[cfg(any(feature = "time", feature = "chrono"))]
    pub next_collection: Option<Date>,
    /// Last collection date and time
    #[cfg(any(feature = "time", feature = "chrono"))]
    pub last_collection: Option<DateTime>,
    /// Additional info or notice if present
    pub info: Option<String>,
}

/// Represents the complete bin collection schedule for a property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyBinSchedule {
    /// Merton property / UPRN ID
    pub property_id: String,
    /// Property address if parsed from the page
    pub address: Option<String>,
    /// List of bin collection services and their schedules
    pub collections: Vec<Collection>,
}
