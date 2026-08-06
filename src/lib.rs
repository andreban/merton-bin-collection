// SPDX-License-Identifier: Apache-2.0

//! `merton-bin-collection`
//!
//! A Rust library for fetching and parsing bin collection schedules and addresses
//! from the Merton Council (FixMyStreet / SocietyWorks) website.

pub mod client;
pub mod error;
pub mod models;
pub mod parser;

pub use client::MertonClient;
pub use error::MertonError;
pub use models::{AddressSearchResult, Collection, PropertyBinSchedule};

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_get_collections() {
        let client = MertonClient::new();
        let result = client.get_collections("https://fixmystreet.merton.gov.uk/waste/4258300").await;
        assert!(result.is_ok(), "Failed live get_collections test: {:?}", result.err());

        let schedule = result.unwrap();
        assert_eq!(schedule.property_id, "4258300");
        assert!(!schedule.collections.is_empty(), "Collections list should not be empty");
        
        println!("Address: {:?}", schedule.address);
        for c in &schedule.collections {
            println!("Service: {}, Next: {:?}", c.service_name, c.next_collection);
        }
    }

    #[tokio::test]
    async fn test_live_search_by_postcode() {
        let client = MertonClient::new();
        let result = client.search_by_postcode("SW19 8ED").await;
        assert!(result.is_ok(), "Failed live postcode search test: {:?}", result.err());

        let addresses = result.unwrap();
        assert!(!addresses.is_empty(), "Address search results should not be empty");
        assert!(addresses.iter().any(|a| a.id == "4258300"));
    }
}
