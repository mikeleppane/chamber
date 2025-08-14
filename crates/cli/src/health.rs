use crate::is_weak_password;
use chamber_vault::{Item, ItemKind, Vault};
use color_eyre::Result;
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug)]
struct HealthReport {
    weak_passwords: Vec<String>,
    reused_passwords: Vec<(String, Vec<String>)>, // password hash -> list of item names
    old_passwords: Vec<(String, i64)>,            // item name -> days old
    short_passwords: Vec<String>,
    common_passwords: Vec<String>,
    total_items: usize,
    password_items: usize,
    security_score: f32,
}

impl HealthReport {
    const fn new() -> Self {
        Self {
            weak_passwords: Vec::new(),
            reused_passwords: Vec::new(),
            old_passwords: Vec::new(),
            short_passwords: Vec::new(),
            common_passwords: Vec::new(),
            total_items: 0,
            password_items: 0,
            security_score: 0.0,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn calculate_security_score(&mut self) {
        if self.password_items == 0 {
            self.security_score = 10.0;
            return;
        }

        let mut score = 10.0;

        // Deduct points for weak passwords (up to 3 points)
        let weak_penalty = (self.weak_passwords.len() as f32 / self.password_items as f32) * 3.0;
        score -= weak_penalty;

        // Deduct points for reused passwords (up to 2.5 points)
        let reused_count: usize = self.reused_passwords.iter().map(|(_, items)| items.len()).sum();
        let reused_penalty = (reused_count as f32 / self.password_items as f32) * 2.5;
        score -= reused_penalty;

        // Deduct points for old passwords (up to 2 points)
        let old_penalty = (self.old_passwords.len() as f32 / self.password_items as f32) * 2.0;
        score -= old_penalty;

        // Deduct points for short passwords (up to 1.5 points)
        let short_penalty = (self.short_passwords.len() as f32 / self.password_items as f32) * 1.5;
        score -= short_penalty;

        // Deduct points for common passwords (up to 1 point)
        let common_penalty = (self.common_passwords.len() as f32 / self.password_items as f32) * 1.0;
        score -= common_penalty;

        self.security_score = score.max(0.0);
    }
}

pub fn handle_health_command(vault: &Vault, detailed: bool) -> Result<()> {
    let items = vault.list_items()?;
    let mut report = analyze_vault_health(&items);
    report.calculate_security_score();

    println!("🏥 Vault Health Report");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Security score with color coding
    let score_color = match report.security_score {
        s if s >= 8.0 => "🟢",
        s if s >= 6.0 => "🟡",
        s if s >= 4.0 => "🟠",
        _ => "🔴",
    };
    println!(
        "{} Overall Security Score: {:.1}/10.0",
        score_color, report.security_score
    );
    println!();

    // Summary
    let total_issues = report.weak_passwords.len()
        + report.reused_passwords.len()
        + report.old_passwords.len()
        + report.short_passwords.len()
        + report.common_passwords.len();

    if total_issues == 0 {
        println!("✅ Excellent! No security issues found.");
        println!("   Your vault maintains good security hygiene.");
        return Ok(());
    }

    println!("📊 Summary:");
    if !report.weak_passwords.is_empty() {
        println!(
            "🔴 {} weak password(s) (missing complexity requirements)",
            report.weak_passwords.len()
        );
    }
    if !report.reused_passwords.is_empty() {
        let reused_count: usize = report.reused_passwords.iter().map(|(_, items)| items.len()).sum();
        println!(
            "🟡 {} reused password(s) across {} items",
            report.reused_passwords.len(),
            reused_count
        );
    }
    if !report.old_passwords.is_empty() {
        println!("🟠 {} password(s) older than 1 year", report.old_passwords.len());
    }
    if !report.short_passwords.is_empty() {
        println!(
            "🟡 {} password(s) shorter than 12 characters",
            report.short_passwords.len()
        );
    }
    if !report.common_passwords.is_empty() {
        println!("🔴 {} common/dictionary password(s)", report.common_passwords.len());
    }
    println!();

    if detailed {
        print_detailed_health_report(&report);
    }

    println!("💡 Recommendations:");
    if !report.weak_passwords.is_empty() {
        println!("   • Update weak passwords with stronger alternatives");
        println!("     Use: chamber generate --complex --length 16");
    }
    if !report.reused_passwords.is_empty() {
        println!("   • Replace reused passwords with unique ones for each account");
    }
    if !report.old_passwords.is_empty() {
        println!("   • Consider updating passwords older than 1 year");
    }
    if !report.short_passwords.is_empty() {
        println!("   • Increase password length to at least 12 characters");
    }
    if !report.common_passwords.is_empty() {
        println!("   • Replace common passwords with randomly generated ones");
    }

    println!();
    println!("🔧 Quick fix: Run 'chamber generate' to create strong passwords");

    Ok(())
}

fn analyze_vault_health(items: &[Item]) -> HealthReport {
    let mut report = HealthReport::new();
    let mut password_hashes: HashMap<String, Vec<String>> = HashMap::new();

    report.total_items = items.len();

    let now = OffsetDateTime::now_utc();
    let one_year_ago = now - time::Duration::days(365);

    for item in items {
        // Only analyze password-type items
        if !matches!(item.kind, ItemKind::Password | ItemKind::ApiKey) {
            continue;
        }

        report.password_items += 1;
        let password = &item.value;

        // Check for weak passwords
        if is_weak_password(password) {
            report.weak_passwords.push(item.name.clone());
        }

        // Check for short passwords
        if password.len() < 12 {
            report.short_passwords.push(item.name.clone());
        }

        // Check for old passwords (older than 1 year)
        if item.updated_at < one_year_ago {
            let days_old = (now - item.updated_at).whole_days();
            report.old_passwords.push((item.name.clone(), days_old));
        }

        // Check for common passwords
        if is_common_password(password) {
            report.common_passwords.push(item.name.clone());
        }

        // Track password reuse
        let password_hash = simple_hash(password);
        password_hashes
            .entry(password_hash)
            .or_default()
            .push(item.name.clone());
    }

    // Find reused passwords (the same hash appears multiple times)
    for (hash, item_names) in password_hashes {
        if item_names.len() > 1 {
            report.reused_passwords.push((hash, item_names));
        }
    }

    report
}

fn print_detailed_health_report(report: &HealthReport) {
    println!("📋 Detailed Analysis:");
    println!();

    if !report.weak_passwords.is_empty() {
        println!("🔴 Weak Passwords ({}):", report.weak_passwords.len());
        for name in &report.weak_passwords {
            println!("   • {name}");
        }
        println!("   Requirements: 10+ chars, uppercase, lowercase, digit, special char");
        println!();
    }

    if !report.reused_passwords.is_empty() {
        println!("🟡 Reused Passwords ({} groups):", report.reused_passwords.len());
        for (i, (_, items)) in report.reused_passwords.iter().enumerate() {
            println!("   Group {}: {} items", i + 1, items.len());
            for item in items {
                println!("     • {item}");
            }
        }
        println!();
    }

    #[allow(clippy::cast_precision_loss)]
    if !report.old_passwords.is_empty() {
        println!("🟠 Old Passwords ({}):", report.old_passwords.len());
        for (name, days) in &report.old_passwords {
            let years = *days as f32 / 365.0;
            println!("   • {name} (last updated {years:.1} years ago)");
        }
        println!();
    }

    if !report.short_passwords.is_empty() {
        println!("🟡 Short Passwords ({}):", report.short_passwords.len());
        for name in &report.short_passwords {
            println!("   • {name}");
        }
        println!();
    }

    if !report.common_passwords.is_empty() {
        println!("🔴 Common Passwords ({}):", report.common_passwords.len());
        for name in &report.common_passwords {
            println!("   • {name}");
        }
        println!();
    }
}

fn is_common_password(password: &str) -> bool {
    // List of common passwords to check against
    const COMMON_PASSWORDS: &[&str] = &[
        "password",
        "123456",
        "password123",
        "admin",
        "qwerty",
        "letmein",
        "welcome",
        "monkey",
        "dragon",
        "123456789",
        "password1",
        "abc123",
        "111111",
        "iloveyou",
        "master",
        "sunshine",
        "princess",
        "football",
        "123123",
        "login",
        "admin123",
        "solo",
        "1234567890",
        "starwars",
        "charlie",
        "aa123456",
        "donald",
        "password12",
        "qwerty123",
    ];

    let lower_password = password.to_lowercase();
    COMMON_PASSWORDS
        .iter()
        .any(|&common| lower_password == common || lower_password.contains(common))
}

fn simple_hash(input: &str) -> String {
    // Simple hash for password comparison (not cryptographic)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
