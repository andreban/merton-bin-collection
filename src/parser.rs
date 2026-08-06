// SPDX-License-Identifier: Apache-2.0

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime};
use scraper::{Html, Selector};
use crate::error::MertonError;
use crate::models::{AddressSearchResult, Collection, PropertyBinSchedule};

/// Parses postcode search results HTML into a list of address options.
pub fn parse_postcode_addresses(html: &str) -> Result<Vec<AddressSearchResult>, MertonError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("select#address option, select[name='address'] option")
        .map_err(|e| MertonError::Parse(format!("Invalid CSS selector: {:?}", e)))?;

    let mut results = Vec::new();

    for option in document.select(&selector) {
        if let Some(id) = option.value().attr("value") {
            let id = id.trim();
            if id.is_empty() {
                continue;
            }
            let text = option.text().collect::<Vec<_>>().join(" ");
            let address = clean_text(&text);
            if !address.is_empty() {
                results.push(AddressSearchResult {
                    id: id.to_string(),
                    address,
                });
            }
        }
    }

    Ok(results)
}

/// Parses the bin schedule HTML fragment/page using current local date as reference.
pub fn parse_bin_schedule(property_id: &str, html: &str) -> Result<PropertyBinSchedule, MertonError> {
    let today = Local::now().date_naive();
    parse_bin_schedule_with_ref_date(property_id, html, today)
}

/// Parses the bin schedule HTML fragment/page with a specific reference date for year resolution.
pub fn parse_bin_schedule_with_ref_date(
    property_id: &str,
    html: &str,
    ref_date: NaiveDate,
) -> Result<PropertyBinSchedule, MertonError> {
    let document = Html::parse_document(html);

    // Extract property address if present
    let address_selector = Selector::parse(".waste__address__property")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;

    let address = document
        .select(&address_selector)
        .next()
        .map(|el| clean_text(&el.text().collect::<Vec<_>>().join(" ")))
        .filter(|a| !a.is_empty());

    // Extract collection grids
    let grid_selector = Selector::parse(".waste-service-grid")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;

    let name_selector = Selector::parse(".waste-service-name, h3")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;
    let row_selector = Selector::parse(".govuk-summary-list__row")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;
    let key_selector = Selector::parse(".govuk-summary-list__key")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;
    let val_selector = Selector::parse(".govuk-summary-list__value")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;
    let p_selector = Selector::parse("p")
        .map_err(|e| MertonError::Parse(format!("Invalid selector: {:?}", e)))?;

    let mut collections = Vec::new();

    for grid in document.select(&grid_selector) {
        let service_name = match grid.select(&name_selector).next() {
            Some(el) => clean_text(&el.text().collect::<Vec<_>>().join(" ")),
            None => continue,
        };

        if service_name.is_empty() {
            continue;
        }

        let mut frequency = None;
        let mut next_collection_raw = None;
        let mut last_collection_raw = None;

        for row in grid.select(&row_selector) {
            let key = row
                .select(&key_selector)
                .next()
                .map(|k| clean_text(&k.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            let val = row
                .select(&val_selector)
                .next()
                .map(|v| clean_text(&v.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();

            if key.eq_ignore_ascii_case("Frequency") {
                frequency = Some(val);
            } else if key.eq_ignore_ascii_case("Next collection") {
                next_collection_raw = Some(val);
            } else if key.eq_ignore_ascii_case("Last collection") {
                last_collection_raw = Some(val);
            }
        }

        let info = grid
            .select(&p_selector)
            .next()
            .map(|p| clean_text(&p.text().collect::<Vec<_>>().join(" ")))
            .filter(|p| !p.is_empty());

        let next_collection = next_collection_raw
            .as_deref()
            .and_then(|s| parse_date_string(s, ref_date));

        let last_collection = last_collection_raw
            .as_deref()
            .and_then(|s| parse_datetime_string(s, ref_date));

        collections.push(Collection {
            service_name,
            frequency,
            next_collection,
            last_collection,
            info,
        });
    }

    Ok(PropertyBinSchedule {
        property_id: property_id.to_string(),
        address,
        collections,
    })
}

/// Helper function to normalize and clean HTML element text.
fn clean_text(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parses date strings such as "Friday 7 August", "Friday 14 August 2026", "Today", or "Tomorrow".
pub fn parse_date_string(s: &str, ref_date: NaiveDate) -> Option<NaiveDate> {
    let s_clean = s.trim();

    if s_clean.eq_ignore_ascii_case("today") {
        return Some(ref_date);
    }
    if s_clean.eq_ignore_ascii_case("tomorrow") {
        return ref_date.succ_opt();
    }

    let replaced = s_clean.replace(',', " ");
    let tokens: Vec<&str> = replaced.split_whitespace().collect();

    let mut day: Option<u32> = None;
    let mut month: Option<u32> = None;
    let mut year: Option<i32> = None;

    for token in &tokens {
        if let Ok(y) = token.parse::<i32>() {
            if (2020..=2100).contains(&y) {
                year = Some(y);
                continue;
            }
        }
        if let Ok(d) = token.parse::<u32>() {
            if (1..=31).contains(&d) {
                day = Some(d);
                continue;
            }
        }
        if let Some(m) = parse_month_name(token) {
            month = Some(m);
        }
    }

    let d = day?;
    let m = month?;
    let y = match year {
        Some(y) => y,
        None => {
            let mut y = ref_date.year();
            if let Some(candidate) = NaiveDate::from_ymd_opt(y, m, d) {
                if candidate < ref_date - chrono::Duration::days(180) {
                    y += 1;
                } else if candidate > ref_date + chrono::Duration::days(180) {
                    y -= 1;
                }
            }
            y
        }
    };

    NaiveDate::from_ymd_opt(y, m, d)
}

/// Parses date-time strings such as "Friday 31 July, at 10:21am" or "Friday 31 July, at 8:46am".
pub fn parse_datetime_string(s: &str, ref_date: NaiveDate) -> Option<NaiveDateTime> {
    let s_clean = s.trim();
    let parts: Vec<&str> = if s_clean.contains(", at ") {
        s_clean.split(", at ").collect()
    } else if s_clean.contains(" at ") {
        s_clean.split(" at ").collect()
    } else {
        vec![s_clean]
    };

    let date_part = parts.first()?;
    let date = parse_date_string(date_part, ref_date)?;

    if parts.len() > 1 {
        let time_part = parts[1].trim();
        if let Some(time) = parse_time_string(time_part) {
            return Some(date.and_time(time));
        }
    }

    date.and_hms_opt(0, 0, 0)
}

fn parse_month_name(s: &str) -> Option<u32> {
    match s.to_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn parse_time_string(s: &str) -> Option<NaiveTime> {
    let s_lower = s.to_lowercase().replace(' ', "");
    let is_pm = s_lower.ends_with("pm");
    let is_am = s_lower.ends_with("am");
    let digits = s_lower.trim_end_matches("am").trim_end_matches("pm");

    let parts: Vec<&str> = digits.split(':').collect();
    let mut hour: u32 = parts.first()?.parse().ok()?;
    let minute: u32 = if parts.len() > 1 {
        parts[1].parse().ok()?
    } else {
        0
    };

    if is_pm && hour < 12 {
        hour += 12;
    } else if is_am && hour == 12 {
        hour = 0;
    }

    NaiveTime::from_hms_opt(hour, minute, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_and_time_strings() {
        let ref_date = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();

        assert_eq!(
            parse_date_string("Friday 7 August", ref_date),
            Some(NaiveDate::from_ymd_opt(2026, 8, 7).unwrap())
        );

        assert_eq!(
            parse_datetime_string("Friday 31 July, at 10:21am", ref_date),
            Some(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap().and_hms_opt(10, 21, 0).unwrap())
        );

        assert_eq!(
            parse_datetime_string("Friday 31 July, at 8:46am", ref_date),
            Some(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap().and_hms_opt(8, 46, 0).unwrap())
        );
    }

    #[test]
    fn test_parse_postcode_addresses() {
        let html = r#"
        <select id="address" name="address">
            <option value=""></option>
            <option value="4258300">2 Gresham Way, Wimbledon, London, SW19 8ED</option>
            <option value="4258302">4 Gresham Way, Wimbledon, London, SW19 8ED</option>
        </select>
        "#;

        let results = parse_postcode_addresses(html).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "4258300");
        assert_eq!(results[0].address, "2 Gresham Way, Wimbledon, London, SW19 8ED");
        assert_eq!(results[1].id, "4258302");
    }

    #[test]
    fn test_parse_bin_schedule() {
        let ref_date = NaiveDate::from_ymd_opt(2026, 8, 6).unwrap();
        let html = r#"
        <dl class="waste__address">
            <dd class="waste__address__property">2 Gresham Way, Wimbledon, London, SW19 8ED</dd>
        </dl>
        <div class="waste-service-grid">
            <div class="waste-service-grid--service-name">
                <h3 class="govuk-heading-m waste-service-name">Food waste</h3>
            </div>
            <div class="waste-service-grid--service-description">
                <dl class="govuk-summary-list">
                    <div class="govuk-summary-list__row">
                        <dt class="govuk-summary-list__key">Frequency</dt>
                        <dd class="govuk-summary-list__value">Every Friday</dd>
                    </div>
                    <div class="govuk-summary-list__row">
                        <dt class="govuk-summary-list__key">Next collection</dt>
                        <dd class="govuk-summary-list__value">Friday 7 August</dd>
                    </div>
                    <div class="govuk-summary-list__row">
                        <dt class="govuk-summary-list__key">Last collection</dt>
                        <dd class="govuk-summary-list__value">Friday 31 July, at 10:21am</dd>
                    </div>
                </dl>
            </div>
        </div>
        "#;

        let schedule = parse_bin_schedule_with_ref_date("4258300", html, ref_date).unwrap();
        assert_eq!(schedule.property_id, "4258300");
        assert_eq!(schedule.address.as_deref(), Some("2 Gresham Way, Wimbledon, London, SW19 8ED"));
        assert_eq!(schedule.collections.len(), 1);
        let c = &schedule.collections[0];
        assert_eq!(c.service_name, "Food waste");
        assert_eq!(c.frequency.as_deref(), Some("Every Friday"));
        assert_eq!(c.next_collection, Some(NaiveDate::from_ymd_opt(2026, 8, 7).unwrap()));
        assert_eq!(
            c.last_collection,
            Some(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap().and_hms_opt(10, 21, 0).unwrap())
        );
    }
}
