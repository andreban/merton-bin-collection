# merton-bin-collection

A Rust library for searching property addresses and pulling bin collection schedules from the Merton Council website ([FixMyStreet / SocietyWorks](https://fixmystreet.merton.gov.uk/waste)).

## Features & Date Libraries

- **Postcode Lookup**: Search for property addresses and UPRN IDs by UK postcode.
- **Schedule Extraction**: Fetch bin collection schedules (Food waste, Mixed recycling, Paper and card, Non-recyclable waste, Bulky waste).
- **Asynchronous Polling**: Automatically handles background page loading and server-side rendering loops.
- **Flexible Inputs**: Accepts both raw property IDs (`4258300`) and full Merton Council URLs (`https://fixmystreet.merton.gov.uk/waste/4258300`).
- **Date Library Support**: Enables date parsing via Cargo features:
  - `time` (**default**): Uses types from the [`time`](https://crates.io/crates/time) crate (`time::Date` and `time::PrimitiveDateTime`).
  - `chrono`: Uses types from the [`chrono`](https://crates.io/crates/chrono) crate (`chrono::NaiveDate` and `chrono::NaiveDateTime`).

### Feature Configuration

By default, the library uses the `time` crate:

```toml
[dependencies]
merton-bin-collection = "0.1"
```

To use `chrono` instead of `time`:

```toml
[dependencies]
merton-bin-collection = { version = "0.1", default-features = false, features = ["chrono"] }
```

## Example Usage

Run the included example using Cargo:

```bash
cargo run --example fetch_schedule
```

### Code Example

```rust
use merton_bin_collection::MertonClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MertonClient::new();

    // 1. Search addresses by postcode
    let addresses = client.search_by_postcode("SW19 8ED").await?;
    for addr in &addresses {
        println!("[{}] {}", addr.id, addr.address);
    }

    // 2. Fetch collection schedule by URL or ID
    let schedule = client
        .get_collections("https://fixmystreet.merton.gov.uk/waste/4258300")
        .await?;

    println!("\nAddress: {:?}", schedule.address);
    for col in schedule.collections {
        println!("Service: {}", col.service_name);
        println!("  Next: {:?}", col.next_collection); // Option<Date>
    }

    Ok(())
}
```
