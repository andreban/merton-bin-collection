// SPDX-License-Identifier: Apache-2.0

use merton_bin_collection::MertonClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MertonClient::new();

    println!("=== Merton Bin Collection Example ===\n");

    // 1. Search for property addresses by postcode
    let postcode = "SW19 8ED";
    println!("1. Searching addresses for postcode: {}", postcode);
    match client.search_by_postcode(postcode).await {
        Ok(addresses) => {
            println!("   Found {} matching address(es):", addresses.len());
            for addr in &addresses {
                println!("   - [{}] {}", addr.id, addr.address);
            }
        }
        Err(err) => eprintln!("   Error searching postcode: {}", err),
    }

    println!("\n-------------------------------------\n");

    // 2. Fetch bin collection schedule for example URL / property ID
    let target_url = "https://fixmystreet.merton.gov.uk/waste/4258300";
    println!("2. Fetching collection schedule for: {}", target_url);

    match client.get_collections(target_url).await {
        Ok(schedule) => {
            println!("   Property ID: {}", schedule.property_id);
            if let Some(addr) = &schedule.address {
                println!("   Address: {}", addr);
            }

            println!("\n   Bin Collection Schedule:");
            for col in &schedule.collections {
                println!("   - Service: {}", col.service_name);
                if let Some(freq) = &col.frequency {
                    println!("     Frequency: {}", freq);
                }
                if let Some(next) = col.next_collection {
                    println!("     Next Collection: {}", next);
                }
                if let Some(last) = col.last_collection {
                    println!("     Last Collection: {}", last);
                }
                if let Some(info) = &col.info {
                    println!("     Info: {}", info);
                }
                println!();
            }
        }
        Err(err) => eprintln!("   Error fetching collections: {}", err),
    }

    Ok(())
}
