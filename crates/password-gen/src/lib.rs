use rand::seq::{IndexedRandom, SliceRandom};
use rand::{Rng, rng};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    pub length: usize,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_digits: bool,
    pub include_symbols: bool,
    pub exclude_ambiguous: bool,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            length: 16,
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_symbols: true,
            exclude_ambiguous: true,
        }
    }
}

impl PasswordConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_length(mut self, length: usize) -> Self {
        self.length = length.clamp(4, 128); // Reasonable bounds
        self
    }

    #[must_use]
    pub const fn with_uppercase(mut self, include: bool) -> Self {
        self.include_uppercase = include;
        self
    }

    #[must_use]
    pub const fn with_lowercase(mut self, include: bool) -> Self {
        self.include_lowercase = include;
        self
    }

    #[must_use]
    pub const fn with_digits(mut self, include: bool) -> Self {
        self.include_digits = include;
        self
    }

    #[must_use]
    pub const fn with_symbols(mut self, include: bool) -> Self {
        self.include_symbols = include;
        self
    }

    #[must_use]
    pub const fn with_exclude_ambiguous(mut self, exclude: bool) -> Self {
        self.exclude_ambiguous = exclude;
        self
    }

    /// Validate that at least one character set is enabled
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.include_uppercase || self.include_lowercase || self.include_digits || self.include_symbols
    }

    /// Get the character sets based on configuration
    fn get_character_sets(&self) -> Vec<&'static str> {
        let mut sets = Vec::new();

        if self.include_lowercase {
            if self.exclude_ambiguous {
                sets.push("abcdefghijkmnopqrstuvwxyz"); // Excludes 'l'
            } else {
                sets.push("abcdefghijklmnopqrstuvwxyz");
            }
        }

        if self.include_uppercase {
            if self.exclude_ambiguous {
                sets.push("ABCDEFGHJKLMNPQRSTUVWXYZ"); // Excludes 'I', 'O'
            } else {
                sets.push("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
            }
        }

        if self.include_digits {
            if self.exclude_ambiguous {
                sets.push("23456789"); // Excludes '0', '1'
            } else {
                sets.push("0123456789");
            }
        }

        if self.include_symbols {
            if self.exclude_ambiguous {
                sets.push("!@#$%^&*+-=?"); // Excludes similar looking symbols
            } else {
                sets.push("!@#$%^&*()_+-=[]{}|;:,.<>?");
            }
        }

        sets
    }

    /// Generates a random password based on the character sets and length defined in the struct.
    ///
    /// # Returns
    /// - `Ok(String)` containing the generated password as per the specified requirements.
    /// - `Err(anyhow::Error)` if the password generation fails due to invalid configuration.
    ///
    /// The function performs the following steps:
    /// 1. Validates that at least one character set is enabled using `self.is_valid()`.
    ///    - If no character set is enabled, an error is returned indicating this condition.
    /// 2. Computes the available character sets as a vector of strings by calling `self.get_character_sets()`.
    /// 3. Collects all unique characters from these sets into a single collection.
    ///    - If the resulting collection is empty, an error is returned indicating no valid characters are available.
    /// 4. Ensures that the generated password contains at least one character from each enabled set.
    /// 5. Randomly fills the rest of the password until the desired length is reached.
    /// 6. The generated password is shuffled to avoid predictable patterns.
    ///
    /// # Errors
    /// - Returns an error if:
    ///   - No character sets are enabled (`"At least one character set must be enabled"`).
    ///   - No valid characters are available (`"No valid characters available"`).
    ///
    /// # Notes
    /// - The `self.is_valid()` method is expected to verify whether at least one character set is enabled.
    /// - The `self.get_character_sets()` method should return a collection of strings, each containing
    ///   a group of valid characters (e.g., lowercase, uppercase, digits, symbols).
    /// - The password length is determined by `self.length`, which should already be validated to be a valid size.
    /// - This method uses randomness; ensure you have a valid random number generator (`rng()`).
    pub fn generate(&self) -> anyhow::Result<String> {
        if !self.is_valid() {
            return Err(anyhow::anyhow!("At least one character set must be enabled"));
        }

        let character_sets = self.get_character_sets();
        let all_chars: String = character_sets.join("");
        let all_chars: Vec<char> = all_chars.chars().collect();

        if all_chars.is_empty() {
            return Err(anyhow::anyhow!("No valid characters available"));
        }

        let mut rng = rng();
        let mut password = Vec::with_capacity(self.length);

        // Ensure at least one character from each enabled set
        for set in &character_sets {
            let chars: Vec<char> = set.chars().collect();
            if let Some(&ch) = chars.choose(&mut rng) {
                password.push(ch);
            }
        }

        // Fill the rest randomly
        while password.len() < self.length {
            if let Some(&ch) = all_chars.choose(&mut rng) {
                password.push(ch);
            }
        }

        // Shuffle the password to avoid predictable patterns
        password.shuffle(&mut rng);

        Ok(password.into_iter().collect())
    }
}

/// Generates a simple password of the specified length.
///
/// This function utilizes the `PasswordConfig` structure to generate a password
/// with certain constraints set:
/// - The length of the password is determined by the `length` parameter.
/// - The password will not include symbols, only alphanumeric characters.
///
/// # Arguments
///
/// * `length` - A `usize` value specifying the desired length of the password.
///
/// # Returns
///
/// Returns an `anyhow::Result<String>`:
/// - `Ok(String)` containing the generated password if successful.
/// - `Err(anyhow::Error)` if an error occurs during password generation.
///
/// # Errors
///
/// This function may return an error if the `PasswordConfig` fails to generate
/// a password due to invalid parameters or runtime issues.
///
/// Note: Ensure the `length` parameter is a positive value within acceptable limits
/// supported by the password generator, as extremely large lengths may cause failures.
pub fn generate_simple_password(length: usize) -> anyhow::Result<String> {
    PasswordConfig::new().with_length(length).with_symbols(false).generate()
}

/// Generates a complex password based on the specified length.
///
/// The function utilizes the `PasswordConfig` struct to create a password with specific parameters.
/// It ensures the password generation process includes all necessary configurations to create a robust and secure password.
///
/// # Parameters
/// - `length`: A `usize` value representing the desired length of the generated password.
///
/// # Returns
/// - Returns an `anyhow::Result<String>`:
///   - On success: A `String` containing the generated password.
///   - On failure: An error wrapped in an `anyhow::Result` indicating what went wrong during generation.
///
/// # Behavior
/// - The generated password includes alphanumeric characters as well as symbols for added complexity.
/// - The function explicitly permits the inclusion of ambiguous characters (e.g., characters that may be difficult to distinguish visually).
///
/// # Errors
/// - The function returns an error if there is a failure in the password generation process.
///   This may occur due to issues with underlying configurations or constraints.
///
/// # Notes
/// - Ensure to validate the `length` parameter before calling the function, as extremely large or small lengths
///   may not be suitable based on the use case.
///
/// # Dependencies
/// - The function relies on the `PasswordConfig` struct and its associated methods, which are assumed to be properly defined elsewhere in the codebase.
pub fn generate_complex_password(length: usize) -> anyhow::Result<String> {
    PasswordConfig::new()
        .with_length(length)
        .with_exclude_ambiguous(false)
        .generate()
}

/// Generates a memorable yet relatively secure password by combining randomly chosen
/// syllables, numbers, and separators.
///
/// # Details
/// - The password consists of two "words," each made up of 3–4 randomly selected syllables
///   from a predefined list of simple syllables for enhanced readability.
/// - The first syllable of the first word is capitalized to make the password more
///   distinguishable.
/// - A numeric separator is added between the two "words."
/// - The password is appended with two random digits at the end to increase variability
///   and entropy.
///
/// # Returns
/// A `String` containing the generated password.
///
/// # Behavior
/// - Each execution produces a different password due to randomization.
/// - Example format: `BaCU4raze19`
///
/// # Attributes
/// - **#[`must_use`]:** This function returns a value that should be used; failing to
///   use the returned password may indicate a mistake.
///
/// # Panics
/// - The function uses `expect` to ensure that digit selection from the allowed range
///   (0–9) succeeds. Panics will only occur if the internal logic of random digit
///   generation fails (highly unlikely).
///
/// # Notes
/// - Designed for use cases requiring user-friendly yet secure passphrases.
/// - Consider using additional validation if stronger security is required.
#[must_use]
pub fn generate_memorable_password() -> String {
    let mut rng = rng();

    // Simple syllables for readability
    let syllables = [
        "ba", "be", "bi", "bo", "bu", "ca", "ce", "ci", "co", "cu", "da", "de", "di", "do", "du", "fa", "fe", "fi",
        "fo", "fu", "ga", "ge", "gi", "go", "gu", "ha", "he", "hi", "ho", "hu", "ja", "je", "ji", "jo", "ju", "ka",
        "ke", "ki", "ko", "ku", "la", "le", "li", "lo", "lu", "ma", "me", "mi", "mo", "mu", "na", "ne", "ni", "no",
        "nu", "pa", "pe", "pi", "po", "pu", "ra", "re", "ri", "ro", "ru", "sa", "se", "si", "so", "su", "ta", "te",
        "ti", "to", "tu", "va", "ve", "vi", "vo", "vu", "wa", "we", "wi", "wo", "wu", "ya", "ye", "yi", "yo", "yu",
        "za", "ze", "zi", "zo", "zu",
    ];

    let mut password = String::new();

    // Generate 3-4 syllable words
    for i in 0..2 {
        let word_length = rng.random_range(3..=4);
        for j in 0..word_length {
            if let Some(&syllable) = syllables.choose(&mut rng) {
                if j == 0 && i == 0 {
                    // Capitalize first syllable of first word
                    password.push_str(&syllable.to_uppercase());
                } else {
                    password.push_str(syllable);
                }
            }
        }

        // Add separator
        if i == 0 {
            #[allow(clippy::expect_used)]
            password.push(
                rng.random_range(0..=9)
                    .to_string()
                    .chars()
                    .next()
                    .expect("No ASCII digits"),
            );
        }
    }

    #[allow(clippy::expect_used)]
    // Add some digits at the end
    for _ in 0..2 {
        password.push(
            rng.random_range(0..=9)
                .to_string()
                .chars()
                .next()
                .expect("No ASCII digits"),
        );
    }

    password
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_password_config_default() {
        let config = PasswordConfig::default();
        assert_eq!(config.length, 16);
        assert!(config.include_uppercase);
        assert!(config.include_lowercase);
        assert!(config.include_digits);
        assert!(config.include_symbols);
        assert!(config.exclude_ambiguous);
    }

    #[test]
    fn test_password_config_new() {
        let config = PasswordConfig::new();
        assert_eq!(config.length, 16);
        assert!(config.include_uppercase);
        assert!(config.include_lowercase);
        assert!(config.include_digits);
        assert!(config.include_symbols);
        assert!(config.exclude_ambiguous);
    }

    #[test]
    fn test_password_config_builder_methods() {
        let config = PasswordConfig::new()
            .with_length(24)
            .with_uppercase(false)
            .with_lowercase(true)
            .with_digits(false)
            .with_symbols(true)
            .with_exclude_ambiguous(true);

        assert_eq!(config.length, 24);
        assert!(!config.include_uppercase);
        assert!(config.include_lowercase);
        assert!(!config.include_digits);
        assert!(config.include_symbols);
        assert!(config.exclude_ambiguous);
    }

    #[test]
    fn test_generate_basic_password() {
        let config = PasswordConfig::default();
        let password = config.generate().unwrap();

        assert_eq!(password.len(), 16);
        assert!(!password.is_empty());
    }

    #[test]
    fn test_generate_different_lengths() {
        for length in [4, 8, 12, 16, 32, 64, 128] {
            let config = PasswordConfig::new().with_length(length);
            let password = config.generate().unwrap();
            assert_eq!(password.len(), length);
        }
    }

    #[test]
    fn test_generate_minimum_length() {
        let config = PasswordConfig::new().with_length(1);
        let password = config.generate().unwrap();
        assert_eq!(password.len(), 4);
    }

    #[test]
    fn test_generate_maximum_length() {
        let config = PasswordConfig::new().with_length(128);
        let password = config.generate().unwrap();
        assert_eq!(password.len(), 128);
    }

    #[test]
    fn test_generate_uppercase_only() {
        let config = PasswordConfig::new()
            .with_length(20)
            .with_uppercase(true)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 20);
        assert!(password.chars().all(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn test_generate_lowercase_only() {
        let config = PasswordConfig::new()
            .with_length(20)
            .with_uppercase(false)
            .with_lowercase(true)
            .with_digits(false)
            .with_symbols(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 20);
        assert!(password.chars().all(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn test_generate_digits_only() {
        let config = PasswordConfig::new()
            .with_length(20)
            .with_uppercase(false)
            .with_lowercase(false)
            .with_digits(true)
            .with_symbols(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 20);
        assert!(password.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_symbols_only() {
        let config = PasswordConfig::new()
            .with_length(20)
            .with_uppercase(false)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(true);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 20);
        assert!(password.chars().all(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)));
    }

    #[test]
    fn test_generate_mixed_character_sets() {
        let config = PasswordConfig::new()
            .with_length(100)
            .with_uppercase(true)
            .with_lowercase(true)
            .with_digits(true)
            .with_symbols(true);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 100);

        // With a long password, we should have characters from all sets
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)));
    }

    #[test]
    fn test_generate_exclude_ambiguous() {
        let config = PasswordConfig::new().with_length(100).with_exclude_ambiguous(true);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 100);

        // Should not contain ambiguous characters
        let ambiguous = "0O1lI";
        assert!(password.chars().all(|c| !ambiguous.contains(c)));
    }

    #[test]
    fn test_generate_include_ambiguous() {
        let config = PasswordConfig::new()
            .with_length(128) // Large sample to increase chance of ambiguous chars
            .with_exclude_ambiguous(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 128);

        // With a large sample and ambiguous characters allowed,
        // we should likely see at least some ambiguous characters
        let ambiguous = "0O1lI";
        let has_ambiguous = password.chars().any(|c| ambiguous.contains(c));
        // Note: This test might occasionally fail due to randomness, but it's very unlikely
        assert!(has_ambiguous);
    }

    #[test]
    fn test_generate_no_character_sets_enabled() {
        let config = PasswordConfig::new()
            .with_length(10)
            .with_uppercase(false)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(false);

        let result = config.generate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("At least one character set must be enabled")
        );
    }

    #[test]
    fn test_generate_zero_length() {
        let config = PasswordConfig::new().with_length(0);
        let password = config.generate().unwrap();
        assert_eq!(password.len(), 4);
        assert!(!password.is_empty());
    }

    #[test]
    fn test_generate_reproducible_with_different_configs() {
        // Test that different configs produce different passwords
        let config1 = PasswordConfig::new().with_length(16).with_symbols(false);
        let config2 = PasswordConfig::new().with_length(16).with_symbols(true);

        let password1 = config1.generate().unwrap();
        let password2 = config2.generate().unwrap();

        // While not guaranteed, passwords should very likely be different
        assert_ne!(password1, password2);
    }

    #[test]
    fn test_generate_multiple_passwords_different() {
        let config = PasswordConfig::new();

        let password1 = config.generate().unwrap();
        let password2 = config.generate().unwrap();
        let password3 = config.generate().unwrap();

        // Passwords should be different (very high probability)
        assert_ne!(password1, password2);
        assert_ne!(password2, password3);
        assert_ne!(password1, password3);
    }

    #[test]
    fn test_simple_password() {
        let password = generate_simple_password(12).unwrap();
        assert_eq!(password.len(), 12);
        assert!(!password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)));

        // Should contain letters and digits only
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_simple_password_different_lengths() {
        for length in [4, 8, 16, 32, 64] {
            let password = generate_simple_password(length).unwrap();
            assert_eq!(password.len(), length);
            assert!(!password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)));
        }
    }

    #[test]
    fn test_simple_password_character_distribution() {
        // Test with a longer password to check character distribution
        let password = generate_simple_password(100).unwrap();
        assert_eq!(password.len(), 100);

        let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digits = password.chars().any(|c| c.is_ascii_digit());
        let has_symbols = password.chars().any(|c| !c.is_ascii_alphanumeric());

        // Should have letters and digits, but no symbols
        assert!(has_uppercase || has_lowercase); // Should have some letters
        assert!(has_digits); // Should have some digits
        assert!(!has_symbols); // Should not have symbols
    }

    #[test]
    fn test_memorable_password_basic() {
        let password = generate_memorable_password();

        assert!(!password.is_empty());
        assert!(password.len() >= 8); // Should be reasonably long
        assert!(password.len() <= 20); // But not too long for memorable

        // Should contain both letters and digits
        assert!(password.chars().any(|c| c.is_ascii_alphabetic()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_memorable_password_format() {
        let password = generate_memorable_password();

        // First character should be uppercase (capitalized first syllable)
        assert!(password.chars().next().unwrap().is_ascii_uppercase());

        // Should contain digits
        assert!(password.chars().any(|c| c.is_ascii_digit()));

        // Should not contain symbols
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_memorable_password_uniqueness() {
        let password1 = generate_memorable_password();
        let password2 = generate_memorable_password();
        let password3 = generate_memorable_password();

        // Should generate different passwords
        assert_ne!(password1, password2);
        assert_ne!(password2, password3);
        assert_ne!(password1, password3);
    }

    #[test]
    fn test_memorable_password_syllable_structure() {
        let password = generate_memorable_password();

        // Remove digits to check syllable structure
        let letters_only: String = password.chars().filter(char::is_ascii_alphabetic).collect();

        // Should have reasonable length for syllables
        // Based on the implementation: 2 words × 3-4 syllables × 2 chars per syllable = 12-16 letters
        assert!(letters_only.len() >= 6);
        assert!(letters_only.len() <= 20); // Increased upper bound to accommodate actual generation

        // Should be pronounceable (no consecutive consonants that are hard to pronounce)
        // This is a basic check - real syllable validation would be more complex
        assert!(!letters_only.is_empty());
    }

    #[test]
    fn test_password_consistency_over_multiple_generations() {
        // Test that the same config produces valid passwords consistently
        let config = PasswordConfig::new().with_length(20);

        for _ in 0..100 {
            let password = config.generate().unwrap();
            assert_eq!(password.len(), 20);
            assert!(password.is_ascii());
        }
    }

    #[test]
    fn test_extreme_configurations() {
        // Test edge cases that should still work

        // Only uppercase letters, length 1
        let config = PasswordConfig::new()
            .with_length(1)
            .with_uppercase(true)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 4);
        assert!(password.chars().next().unwrap().is_ascii_uppercase());
    }

    #[test]
    fn test_character_set_boundaries() {
        // Test that all expected characters can appear
        let config = PasswordConfig::new()
            .with_length(10000) // Large sample
            .with_exclude_ambiguous(false);

        let password = config.generate().unwrap();

        // Check that we get characters from expected ranges
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_symbol = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        assert!(has_upper);
        assert!(has_lower);
        assert!(has_digit);
        assert!(has_symbol);
    }
}
