use crate::{get_type_emoji, is_weak_password};
use chamber_vault::Vault;
use time::{Duration, OffsetDateTime};

#[allow(clippy::too_many_lines)]
pub fn handle_stats_command(vault: &Vault) -> color_eyre::Result<()> {
    let items = vault.list_items()?;

    // Get vault information if possible
    let vault_name = match vault.get_vault_id() {
        Ok(Some(id)) => format!(" ({id})"),
        Ok(None) => " (Default Vault)".to_string(),
        Err(_) => String::new(),
    };

    if items.is_empty() {
        println!("📊 Vault Statistics{vault_name}");
        println!("══════════════════");
        println!("No items found in vault.");
        return Ok(());
    }

    // Rest of the statistics code remains the same...
    let mut type_counts = std::collections::HashMap::new();
    let mut weak_passwords = Vec::new();
    let mut old_items = Vec::new();
    let mut recently_updated = Vec::new();
    let mut duplicate_names = Vec::new();
    let mut password_lengths = Vec::new();

    let now = OffsetDateTime::now_utc();
    let cutoff_date = now - Duration::days(180);
    let recent_cutoff = now - Duration::days(30);

    // Check for duplicate names
    let mut name_counts = std::collections::HashMap::new();
    for item in &items {
        let count = name_counts.entry(&item.name).or_insert(0);
        *count += 1;
    }

    for (name, count) in &name_counts {
        if *count > 1 {
            duplicate_names.push((name.as_str(), *count));
        }
    }

    for item in &items {
        // Count by type
        let count = type_counts.entry(item.kind).or_insert(0);
        *count += 1;

        // Check for weak passwords and collect password lengths
        if item.kind == chamber_vault::ItemKind::Password {
            password_lengths.push(item.value.len());
            if is_weak_password(&item.value) {
                weak_passwords.push(&item.name);
            }
        }

        // Check for old items (older than 180 days)
        if item.created_at < cutoff_date {
            old_items.push((&item.name, item.created_at));
        }

        // Check for recently updated items (within 30 days)
        if item.updated_at > recent_cutoff {
            recently_updated.push((&item.name, item.updated_at));
        }
    }

    // Display statistics with vault name
    println!("📊 Vault Statistics{vault_name}");
    println!("══════════════════");
    println!();

    // Add vault location info
    println!("📍 Vault Info:");
    println!("──────────────");
    println!("  📂 Database Path: {}", vault.db_path().display());
    println!();

    // General overview
    println!("📋 Overview:");
    println!("────────────");
    println!("  📊 Total Items: {}", items.len());

    if let Some(oldest) = items.iter().min_by_key(|item| item.created_at) {
        let vault_age = now - oldest.created_at;
        println!("  📅 Vault Age: {} days (oldest item)", vault_age.whole_days());
    }

    if let Some(newest) = items.iter().max_by_key(|item| item.updated_at) {
        let last_activity = now - newest.updated_at;
        println!("  🕒 Last Activity: {} days ago", last_activity.whole_days());
    }

    println!("  🔄 Recently Updated (30 days): {}", recently_updated.len());
    println!();

    // Items per vault type
    println!("📁 Items by Type:");
    println!("─────────────────");
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

    #[allow(clippy::cast_precision_loss)]
    for (item_type, count) in sorted_types {
        let percentage = (f64::from(*count) / items.len() as f64) * 100.0;
        println!(
            "  {} {}: {} ({:.1}%)",
            get_type_emoji(*item_type),
            item_type.display_name(),
            count,
            percentage
        );
    }
    println!();

    // Password security analysis
    if !password_lengths.is_empty() {
        println!("🔐 Password Security:");
        println!("────────────────────");

        #[allow(clippy::cast_precision_loss)]
        let avg_length = password_lengths.iter().sum::<usize>() as f64 / password_lengths.len() as f64;
        let min_length = password_lengths.iter().min().unwrap_or(&0);
        let max_length = password_lengths.iter().max().unwrap_or(&0);

        println!("  📏 Average Length: {avg_length:.1} characters");
        println!("  📐 Length Range: {min_length} - {max_length} characters");

        let strong_count = password_lengths.len() - weak_passwords.len();
        #[allow(clippy::cast_precision_loss)]
        let strong_percentage = (strong_count as f64 / password_lengths.len() as f64) * 100.0;

        println!("  💪 Strong Passwords: {strong_count} ({strong_percentage:.1}%)");
        println!(
            "  🔓 Weak Passwords: {} ({:.1}%)",
            weak_passwords.len(),
            100.0 - strong_percentage
        );
        println!();
    }

    // Weak passwords details
    println!("🔓 Weak Passwords:");
    println!("──────────────────");
    if weak_passwords.is_empty() {
        println!("  ✅ No weak passwords found!");
    } else {
        println!("  ❌ Found {} weak password(s):", weak_passwords.len());
        for name in &weak_passwords {
            println!("    • {name}");
        }
        println!("  💡 Strong passwords should have at least 10 characters with");
        println!("     uppercase, lowercase, numbers, and special characters.");
    }
    println!();

    // Data quality issues
    println!("🔍 Data Quality:");
    println!("────────────────");

    // Duplicate names
    if duplicate_names.is_empty() {
        println!("  ✅ No duplicate names found!");
    } else {
        println!("  ⚠️  Found {} duplicate name(s):", duplicate_names.len());
        duplicate_names.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        for (name, count) in &duplicate_names {
            println!("    • '{name}' appears {count} times");
        }
    }
    println!();

    // Age analysis
    println!("⏰ Age Analysis:");
    println!("───────────────");

    let new_items = items.iter().filter(|item| item.created_at > recent_cutoff).count();
    let medium_items = items
        .iter()
        .filter(|item| item.created_at <= recent_cutoff && item.created_at >= cutoff_date)
        .count();

    println!("  🆕 New Items (< 30 days): {new_items}");
    println!("  📅 Medium Age (30-180 days): {medium_items}");
    println!("  ⏳ Old Items (> 180 days): {}", old_items.len());

    if !old_items.is_empty() {
        println!("  ⚠️  Oldest items:");
        let mut sorted_old_items = old_items.clone();
        sorted_old_items.sort_by(|a, b| a.1.cmp(&b.1)); // Sort by date ascending (oldest first)

        for (name, created_at) in sorted_old_items.iter().take(5) {
            let age = now - *created_at;
            println!("    • {} ({} days old)", name, age.whole_days());
        }

        if sorted_old_items.len() > 5 {
            println!("    ... and {} more", sorted_old_items.len() - 5);
        }

        println!("  💡 Consider updating or reviewing old items for security.");
    }

    println!();

    // Activity summary
    if !recently_updated.is_empty() {
        println!("🔄 Recent Activity:");
        println!("──────────────────");

        let mut sorted_recent = recently_updated;
        sorted_recent.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by date descending (most recent first)

        println!("  📝 Recently updated items:");
        for (name, updated_at) in sorted_recent.iter().take(5) {
            let age = now - *updated_at;
            println!("    • {} ({} days ago)", name, age.whole_days());
        }

        if sorted_recent.len() > 5 {
            println!("    ... and {} more", sorted_recent.len() - 5);
        }
        println!();
    }

    // Security recommendations
    println!("💡 Security Recommendations:");
    println!("────────────────────────────");

    let mut recommendations = Vec::new();

    if !weak_passwords.is_empty() {
        recommendations.push(format!("Update {} weak password(s)", weak_passwords.len()));
    }

    if !old_items.is_empty() {
        recommendations.push(format!("Review {} old item(s) (>180 days)", old_items.len()));
    }

    if !duplicate_names.is_empty() {
        recommendations.push(format!("Resolve {} duplicate name(s)", duplicate_names.len()));
    }

    let very_old_items = items
        .iter()
        .filter(|item| item.created_at < (now - Duration::days(365)))
        .count();

    if very_old_items > 0 {
        recommendations.push(format!("Consider rotating {very_old_items} very old item(s) (>1 year)"));
    }

    if recommendations.is_empty() {
        println!("  ✅ Your vault looks healthy! No immediate actions needed.");
    } else {
        for (i, rec) in recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }
    }

    Ok(())
}
