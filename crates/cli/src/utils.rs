use chamber_vault::{Item, ItemKind};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use regex::Regex;
use std::str::FromStr;
use time::{Duration, OffsetDateTime};

/// Parse human-readable time expressions like "1 week ago", "3 days ago"
///
/// Parse human-readable time expressions like "1 week ago", "3 days ago"
/// Parse human-readable time expressions like "1 week ago", "3 days ago"
pub fn parse_time_expression(expr: &str) -> Result<OffsetDateTime> {
    let expr = expr.to_lowercase();
    let now = OffsetDateTime::now_utc();

    // Match patterns like "1 week ago", "3 days ago", "2 hours ago"
    // Note: We use \d+ (not -?\d+) to explicitly reject negative numbers
    let re = Regex::new(r"^(\d+)\s+(second|minute|hour|day|week|month|year)s?\s+ago$")
        .map_err(|e| eyre!("Regex error: {}", e))?;

    if let Some(caps) = re.captures(&expr) {
        let number: i64 = caps[1]
            .parse()
            .map_err(|_| eyre!("Invalid number in time expression"))?;

        // Additional validation to ensure non-negative numbers only
        if number < 0 {
            return Err(eyre!("Time expression must use non-negative numbers"));
        }

        let unit = &caps[2];

        let duration = match unit {
            "second" => Duration::seconds(number),
            "minute" => Duration::minutes(number),
            "hour" => Duration::hours(number),
            "day" => Duration::days(number),
            "week" => Duration::weeks(number),
            "month" => Duration::days(number * 30), // Approximate
            "year" => Duration::days(number * 365), // Approximate
            _ => return Err(eyre!("Unknown time unit: {}", unit)),
        };

        return Ok(now - duration);
    }

    // Try parsing absolute dates
    if let Ok(date) = time::Date::parse(&expr, &time::format_description::well_known::Iso8601::DEFAULT) {
        return Ok(date.midnight().assume_utc());
    }

    // Try parsing RFC3339 format
    if let Ok(datetime) = OffsetDateTime::parse(&expr, &time::format_description::well_known::Rfc3339) {
        return Ok(datetime);
    }

    Err(eyre!("Unable to parse time expression: '{}'", expr))
}

/// Check if a name matches a wildcard pattern
pub fn matches_wildcard_pattern(name: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return name.is_empty();
    }

    // Convert wildcard pattern to regex
    let mut regex_pattern = String::new();
    let chars: Vec<char> = pattern.chars().collect();

    for ch in chars {
        match ch {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            '.' => regex_pattern.push_str(r"\."),
            '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '+' | '\\' => {
                // Escape special regex characters
                regex_pattern.push('\\');
                regex_pattern.push(ch);
            }
            _ => regex_pattern.push(ch),
        }
    }
    // Create case-insensitive regex
    let full_pattern = format!("(?i)^{regex_pattern}$");

    if let Ok(regex) = Regex::new(&full_pattern) {
        regex.is_match(name)
    } else {
        // Fallback to simple case-insensitive contains
        let pattern_lower = pattern.replace(['*', '?'], "");
        name.to_lowercase().contains(&pattern_lower.to_lowercase())
    }
}

/// Filter and sort items based on list criteria
pub fn filter_and_sort_items(
    items: Vec<Item>,
    item_type: Option<&str>,
    since: Option<&str>,
    recent: Option<usize>,
    name_pattern: Option<&str>,
) -> Result<Vec<Item>> {
    let mut filtered_items = items;

    // Filter by type
    if let Some(filter_type) = item_type {
        let target_kind = ItemKind::from_str(filter_type)?;
        filtered_items.retain(|item| item.kind == target_kind);
    }

    // Filter by date
    if let Some(since_expr) = since {
        let since_date = parse_time_expression(since_expr)?;
        filtered_items.retain(|item| item.created_at >= since_date);
    }

    // Filter by name pattern
    if let Some(pattern) = name_pattern {
        filtered_items.retain(|item| matches_wildcard_pattern(&item.name, pattern));
    }

    // Sort by creation date (newest first)
    filtered_items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Limit to recent items if specified
    if let Some(limit) = recent {
        filtered_items.truncate(limit);
    }

    Ok(filtered_items)
}

/// Format relative time like "2 hours ago", "3 days ago"
pub fn format_relative_time(datetime: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - datetime;

    let total_seconds = duration.whole_seconds();

    if total_seconds < 60 {
        "just now".to_string()
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        format!("{} minute{} ago", minutes, if minutes == 1 { "" } else { "s" })
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if total_seconds < 2_592_000 {
        let days = total_seconds / 86400;
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if total_seconds < 31_536_000 {
        let months = total_seconds / 2_592_000;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = total_seconds / 31_536_000;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    // Helper function to create test items
    fn create_test_item(name: &str, kind: ItemKind, days_ago: i64) -> Item {
        let created_at = OffsetDateTime::now_utc() - Duration::days(days_ago);
        Item {
            id: 1,
            name: name.to_string(),
            kind,
            value: "test_value".to_string(),
            created_at,
            updated_at: created_at,
        }
    }

    // Tests for parse_time_expression
    mod parse_time_expression_tests {
        use super::*;

        #[test]
        fn test_parse_valid_time_expressions() {
            let now = OffsetDateTime::now_utc();

            // Test seconds
            let result = parse_time_expression("30 seconds ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_seconds() >= 29 && (now - parsed).whole_seconds() <= 31);

            // Test minutes
            let result = parse_time_expression("5 minutes ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_minutes() >= 4 && (now - parsed).whole_minutes() <= 6);

            // Test hours
            let result = parse_time_expression("2 hours ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_hours() >= 1 && (now - parsed).whole_hours() <= 3);

            // Test days
            let result = parse_time_expression("3 days ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_days() >= 2 && (now - parsed).whole_days() <= 4);

            // Test weeks
            let result = parse_time_expression("1 week ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_days() >= 6 && (now - parsed).whole_days() <= 8);

            // Test months (approximate)
            let result = parse_time_expression("2 months ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_days() >= 59 && (now - parsed).whole_days() <= 61);

            // Test years (approximate)
            let result = parse_time_expression("1 year ago");
            assert!(result.is_ok());
            let parsed = result.unwrap();
            assert!((now - parsed).whole_days() >= 364 && (now - parsed).whole_days() <= 366);
        }

        #[test]
        fn test_parse_plural_forms() {
            // Test that both singular and plural forms work
            let result1 = parse_time_expression("1 day ago");
            let result2 = parse_time_expression("1 days ago");
            assert!(result1.is_ok());
            assert!(result2.is_ok());
        }

        #[test]
        fn test_parse_case_insensitive() {
            let result1 = parse_time_expression("1 DAY AGO");
            let result2 = parse_time_expression("1 Day Ago");
            let result3 = parse_time_expression("1 day ago");

            assert!(result1.is_ok());
            assert!(result2.is_ok());
            assert!(result3.is_ok());
        }

        #[test]
        fn test_parse_rfc3339_format() {
            let rfc3339_date = "2024-01-15T10:30:00Z";
            let result = parse_time_expression(rfc3339_date);
            assert!(result.is_ok());

            let parsed = result.unwrap();
            let expected = datetime!(2024-01-15 10:30:00 UTC);
            assert_eq!(parsed, expected);
        }

        #[test]
        fn test_parse_invalid_expressions() {
            // Invalid format
            assert!(parse_time_expression("invalid expression").is_err());
            assert!(parse_time_expression("").is_err());
            assert!(parse_time_expression("ago").is_err());
            assert!(parse_time_expression("1").is_err());
            assert!(parse_time_expression("day ago").is_err());

            // Invalid number
            assert!(parse_time_expression("abc days ago").is_err());
            assert!(parse_time_expression("-5 days ago").is_err());

            // Invalid unit
            assert!(parse_time_expression("1 centuries ago").is_err());
        }

        #[test]
        fn test_parse_edge_cases() {
            // Zero values
            let result = parse_time_expression("0 days ago");
            assert!(result.is_ok());

            // Large numbers
            let result = parse_time_expression("9999 days ago");
            assert!(result.is_ok());
        }
    }

    // Tests for matches_wildcard_pattern
    mod wildcard_pattern_tests {
        use super::*;

        #[test]
        fn test_exact_match() {
            assert!(matches_wildcard_pattern("hello", "hello"));
            assert!(matches_wildcard_pattern("GitHub", "GitHub"));
            assert!(!matches_wildcard_pattern("hello", "world"));
        }

        #[test]
        fn test_case_insensitive_matching() {
            assert!(matches_wildcard_pattern("Hello", "hello"));
            assert!(matches_wildcard_pattern("HELLO", "hello"));
            assert!(matches_wildcard_pattern("hElLo", "HeLlO"));
            assert!(matches_wildcard_pattern("GitHub Token", "github token"));
            assert!(matches_wildcard_pattern("MY-PASSWORD", "my-password"));
        }

        #[test]
        fn test_star_wildcard() {
            // Star at the end
            assert!(matches_wildcard_pattern("GitHub Token", "GitHub*"));
            assert!(matches_wildcard_pattern("github-api-key", "github*"));
            assert!(matches_wildcard_pattern("test", "test*"));

            // Star at the beginning
            assert!(matches_wildcard_pattern("MyPassword", "*Password"));
            assert!(matches_wildcard_pattern("api-key-github", "*github"));
            assert!(matches_wildcard_pattern("password", "*word"));

            // Star in the middle
            assert!(matches_wildcard_pattern("GitHub API Token", "GitHub*Token"));
            assert!(matches_wildcard_pattern("my-secret-key", "my*key"));

            // Multiple stars
            assert!(matches_wildcard_pattern("GitHub API Secret Key", "Git*API*Key"));
            assert!(matches_wildcard_pattern("a-b-c-d", "*-*-*"));
        }

        #[test]
        fn test_question_wildcard() {
            // Single character replacement
            assert!(matches_wildcard_pattern("test1", "test?"));
            assert!(matches_wildcard_pattern("testA", "test?"));
            assert!(matches_wildcard_pattern("test_", "test?"));

            // Multiple question marks
            assert!(matches_wildcard_pattern("test12", "test??"));
            assert!(matches_wildcard_pattern("testAB", "test??"));

            // Question mark doesn't match multiple characters
            assert!(!matches_wildcard_pattern("test12", "test?"));
            assert!(!matches_wildcard_pattern("", "?"));
        }

        #[test]
        fn test_combined_wildcards() {
            assert!(matches_wildcard_pattern("GitHub API v1", "Git*v?"));
            assert!(matches_wildcard_pattern("test-key-123", "test?key*"));
            assert!(matches_wildcard_pattern("api.secret.v2", "api*v?"));
        }

        #[test]
        fn test_special_characters() {
            // Dots should be treated literally (not as regex wildcards)
            assert!(matches_wildcard_pattern("test.com", "test.com"));
            assert!(matches_wildcard_pattern("api.key.v1", "api.key*"));
            assert!(!matches_wildcard_pattern("testXcom", "test.com"));

            // Other special regex characters
            assert!(matches_wildcard_pattern("key[1]", "key[1]"));
            assert!(matches_wildcard_pattern("test(1)", "test(1)"));
            assert!(matches_wildcard_pattern("price$100", "price$*"));
            assert!(matches_wildcard_pattern("start^end", "*^*"));
        }

        #[test]
        fn test_edge_cases() {
            // Empty strings
            assert!(matches_wildcard_pattern("", ""));
            assert!(!matches_wildcard_pattern("test", ""));
            assert!(!matches_wildcard_pattern("", "test"));

            // Only wildcards
            assert!(matches_wildcard_pattern("anything", "*"));
            assert!(matches_wildcard_pattern("a", "?"));
            assert!(matches_wildcard_pattern("ab", "??"));

            // No match cases
            assert!(!matches_wildcard_pattern("GitHub Token", "BitBucket*"));
            assert!(!matches_wildcard_pattern("short", "verylongpattern"));
        }

        #[test]
        fn test_real_world_examples() {
            // Based on the vault export data
            assert!(matches_wildcard_pattern("assdfsf", "*sdf*"));
            assert!(matches_wildcard_pattern("MY-PASSWORD", "MY*"));
            assert!(matches_wildcard_pattern("MY-PASSWORD", "*PASSWORD"));
            assert!(matches_wildcard_pattern("MY-PASSWORD", "MY-*"));

            // Common patterns users might search for
            assert!(matches_wildcard_pattern("github-personal-token", "github*"));
            assert!(matches_wildcard_pattern("aws-secret-key", "*secret*"));
            assert!(matches_wildcard_pattern("db-password-prod", "*password*"));
            assert!(matches_wildcard_pattern("api-key-v2", "api-key*"));
        }
    }

    // Tests for filter_and_sort_items
    mod filter_and_sort_tests {
        use super::*;

        fn create_sample_items() -> Vec<Item> {
            vec![
                create_test_item("password1", ItemKind::Password, 1),
                create_test_item("password2", ItemKind::Password, 3),
                create_test_item("api-key1", ItemKind::ApiKey, 2),
                create_test_item("api-key2", ItemKind::ApiKey, 5),
                create_test_item("note1", ItemKind::Note, 0),
                create_test_item("github-token", ItemKind::ApiKey, 1),
                create_test_item("MY-PASSWORD", ItemKind::Password, 4),
            ]
        }

        #[test]
        fn test_no_filters() {
            let items = create_sample_items();
            let result = filter_and_sort_items(items.clone(), None, None, None, None).unwrap();

            // Should return all items, sorted by creation date (newest first)
            assert_eq!(result.len(), items.len());

            // Verify sorting (newest first)
            for i in 1..result.len() {
                assert!(result[i - 1].created_at >= result[i].created_at);
            }
        }

        #[test]
        fn test_filter_by_type() {
            let items = create_sample_items();

            // Filter by password
            let result = filter_and_sort_items(items.clone(), Some("password"), None, None, None).unwrap();

            assert_eq!(result.len(), 3); // password1, password2, MY-PASSWORD
            assert!(result.iter().all(|item| item.kind == ItemKind::Password));

            // Filter by apikey
            let result = filter_and_sort_items(items.clone(), Some("apikey"), None, None, None).unwrap();

            assert_eq!(result.len(), 3); // api-key1, api-key2, github-token
            assert!(result.iter().all(|item| item.kind == ItemKind::ApiKey));

            // Filter by note
            let result = filter_and_sort_items(items, Some("note"), None, None, None).unwrap();

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].name, "note1");
        }

        #[test]
        fn test_filter_by_invalid_type() {
            let items = create_sample_items();
            let result = filter_and_sort_items(items, Some("invalidtype"), None, None, None);

            assert!(result.is_err());
        }

        #[test]
        fn test_filter_by_name_pattern() {
            let items = create_sample_items();

            // Filter by pattern "password*"
            let result = filter_and_sort_items(items.clone(), None, None, None, Some("password*")).unwrap();

            assert_eq!(result.len(), 2); // password1, password2

            // Filter by pattern "*api*"
            let result = filter_and_sort_items(items.clone(), None, None, None, Some("*api*")).unwrap();

            assert_eq!(result.len(), 2); // api-key1, api-key2

            // Filter by pattern "MY*"
            let result = filter_and_sort_items(items, None, None, None, Some("MY*")).unwrap();

            assert_eq!(result.len(), 1);
            assert_eq!(result[0].name, "MY-PASSWORD");
        }

        #[test]
        fn test_filter_by_recent() {
            let items = create_sample_items();

            // Get 3 most recent items
            let result = filter_and_sort_items(items, None, None, Some(3), None).unwrap();

            assert_eq!(result.len(), 3);

            // Should be sorted by newest first
            for i in 1..result.len() {
                assert!(result[i - 1].created_at >= result[i].created_at);
            }
        }

        #[test]
        fn test_filter_by_since() {
            let items = create_sample_items();

            // Filter by "2 days ago"
            let result = filter_and_sort_items(items, None, Some("2 days ago"), None, None).unwrap();

            // Should include items created in the last 2 days (0, 1 days ago)
            let expected_items: Vec<_> = result
                .iter()
                .filter(|item| {
                    let days_ago = (OffsetDateTime::now_utc() - item.created_at).whole_days();
                    days_ago <= 2
                })
                .collect();

            assert_eq!(result.len(), expected_items.len());
        }

        #[test]
        fn test_filter_by_invalid_since() {
            let items = create_sample_items();
            let result = filter_and_sort_items(items, None, Some("invalid time expression"), None, None);

            assert!(result.is_err());
        }

        #[test]
        fn test_combined_filters() {
            let items = create_sample_items();

            // Combine type and name pattern filters
            let result = filter_and_sort_items(items.clone(), Some("password"), None, None, Some("password*")).unwrap();

            assert_eq!(result.len(), 2); // password1, password2
            assert!(
                result
                    .iter()
                    .all(|item| { item.kind == ItemKind::Password && item.name.starts_with("password") })
            );

            // Combine all filters
            let result =
                filter_and_sort_items(items, Some("apikey"), Some("3 days ago"), Some(2), Some("*key*")).unwrap();

            // Should filter by type=apikey, since=3 days ago, name=*key*, and limit to 2 items
            assert!(result.len() <= 2);
            assert!(result.iter().all(|item| item.kind == ItemKind::ApiKey));
        }

        #[test]
        fn test_empty_results() {
            let items = create_sample_items();

            // Filter that should return no results
            let result = filter_and_sort_items(items, None, None, None, Some("nonexistent*")).unwrap();

            assert!(result.is_empty());
        }
    }

    // Tests for format_relative_time
    mod format_relative_time_tests {
        use super::*;

        #[test]
        fn test_just_now() {
            let now = OffsetDateTime::now_utc();
            assert_eq!(format_relative_time(now), "just now");

            let thirty_seconds_ago = now - Duration::seconds(30);
            assert_eq!(format_relative_time(thirty_seconds_ago), "just now");
        }

        #[test]
        fn test_minutes_ago() {
            let now = OffsetDateTime::now_utc();

            let one_minute_ago = now - Duration::minutes(1);
            assert_eq!(format_relative_time(one_minute_ago), "1 minute ago");

            let five_minutes_ago = now - Duration::minutes(5);
            assert_eq!(format_relative_time(five_minutes_ago), "5 minutes ago");

            let fifty_nine_minutes_ago = now - Duration::minutes(59);
            assert_eq!(format_relative_time(fifty_nine_minutes_ago), "59 minutes ago");
        }

        #[test]
        fn test_hours_ago() {
            let now = OffsetDateTime::now_utc();

            let one_hour_ago = now - Duration::hours(1);
            assert_eq!(format_relative_time(one_hour_ago), "1 hour ago");

            let five_hours_ago = now - Duration::hours(5);
            assert_eq!(format_relative_time(five_hours_ago), "5 hours ago");

            let twenty_three_hours_ago = now - Duration::hours(23);
            assert_eq!(format_relative_time(twenty_three_hours_ago), "23 hours ago");
        }

        #[test]
        fn test_days_ago() {
            let now = OffsetDateTime::now_utc();

            let one_day_ago = now - Duration::days(1);
            assert_eq!(format_relative_time(one_day_ago), "1 day ago");

            let seven_days_ago = now - Duration::days(7);
            assert_eq!(format_relative_time(seven_days_ago), "7 days ago");

            let twenty_nine_days_ago = now - Duration::days(29);
            assert_eq!(format_relative_time(twenty_nine_days_ago), "29 days ago");
        }

        #[test]
        fn test_months_ago() {
            let now = OffsetDateTime::now_utc();

            let one_month_ago = now - Duration::days(30);
            assert_eq!(format_relative_time(one_month_ago), "1 month ago");

            let six_months_ago = now - Duration::days(180);
            assert_eq!(format_relative_time(six_months_ago), "6 months ago");

            let eleven_months_ago = now - Duration::days(330);
            assert_eq!(format_relative_time(eleven_months_ago), "11 months ago");
        }

        #[test]
        fn test_years_ago() {
            let now = OffsetDateTime::now_utc();

            let one_year_ago = now - Duration::days(365);
            assert_eq!(format_relative_time(one_year_ago), "1 year ago");

            let five_years_ago = now - Duration::days(365 * 5);
            assert_eq!(format_relative_time(five_years_ago), "5 years ago");
        }

        #[test]
        fn test_future_dates() {
            let now = OffsetDateTime::now_utc();
            let future = now + Duration::hours(1);

            // Future dates should still work (might show as "just now" or negative)
            let result = format_relative_time(future);
            assert!(result == "just now" || result.contains("ago"));
        }

        #[test]
        fn test_edge_cases() {
            let now = OffsetDateTime::now_utc();

            // Exactly 1 hour boundary
            let exactly_one_hour = now - Duration::seconds(3600);
            assert_eq!(format_relative_time(exactly_one_hour), "1 hour ago");

            // Exactly 1 day boundary
            let exactly_one_day = now - Duration::seconds(86400);
            assert_eq!(format_relative_time(exactly_one_day), "1 day ago");
        }

        #[test]
        fn test_singular_vs_plural() {
            let now = OffsetDateTime::now_utc();

            // Singular forms
            assert_eq!(format_relative_time(now - Duration::minutes(1)), "1 minute ago");
            assert_eq!(format_relative_time(now - Duration::hours(1)), "1 hour ago");
            assert_eq!(format_relative_time(now - Duration::days(1)), "1 day ago");

            // Plural forms
            assert_eq!(format_relative_time(now - Duration::minutes(2)), "2 minutes ago");
            assert_eq!(format_relative_time(now - Duration::hours(2)), "2 hours ago");
            assert_eq!(format_relative_time(now - Duration::days(2)), "2 days ago");
        }
    }
}
