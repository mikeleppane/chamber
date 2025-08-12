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

    #[test]
    fn test_complex_password() {
        let password = generate_complex_password(16).unwrap();
        assert_eq!(password.len(), 16);

        // Complex passwords should include ambiguous characters when exclude_ambiguous is false
        // Test with a larger sample to increase probability
        let large_password = generate_complex_password(100).unwrap();
        let ambiguous = "0O1lI";
        let has_ambiguous = large_password.chars().any(|c| ambiguous.contains(c));
        assert!(has_ambiguous, "Complex password should include ambiguous characters");
    }

    #[test]
    fn test_complex_password_different_lengths() {
        for length in [4, 8, 16, 32, 64] {
            let password = generate_complex_password(length).unwrap();
            assert_eq!(password.len(), length);
            assert!(!password.is_empty());
        }
    }

    #[test]
    fn test_is_valid_method() {
        // Test valid configurations
        let valid_configs = [
            PasswordConfig::new(),
            PasswordConfig::new()
                .with_uppercase(true)
                .with_lowercase(false)
                .with_digits(false)
                .with_symbols(false),
            PasswordConfig::new()
                .with_uppercase(false)
                .with_lowercase(true)
                .with_digits(false)
                .with_symbols(false),
            PasswordConfig::new()
                .with_uppercase(false)
                .with_lowercase(false)
                .with_digits(true)
                .with_symbols(false),
            PasswordConfig::new()
                .with_uppercase(false)
                .with_lowercase(false)
                .with_digits(false)
                .with_symbols(true),
        ];

        for config in valid_configs {
            assert!(config.is_valid(), "Configuration should be valid: {config:?}");
        }

        // Test invalid configuration
        let invalid_config = PasswordConfig::new()
            .with_uppercase(false)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(false);
        assert!(
            !invalid_config.is_valid(),
            "Configuration with no character sets should be invalid"
        );
    }

    #[test]
    fn test_length_clamping_upper_bound() {
        let config = PasswordConfig::new().with_length(200); // Above 128
        let password = config.generate().unwrap();
        assert_eq!(password.len(), 128); // Should be clamped to 128
    }

    #[test]
    fn test_length_clamping_comprehensive() {
        let test_cases = [
            (0, 4),      // Below minimum
            (1, 4),      // Below minimum
            (3, 4),      // Below minimum
            (4, 4),      // Minimum
            (10, 10),    // Normal
            (64, 64),    // Normal
            (128, 128),  // Maximum
            (150, 128),  // Above maximum
            (1000, 128), // Way above maximum
        ];

        for (input, expected) in test_cases {
            let config = PasswordConfig::new().with_length(input);
            assert_eq!(
                config.length, expected,
                "Length {input} should be clamped to {expected}"
            );

            let password = config.generate().unwrap();
            assert_eq!(password.len(), expected);
        }
    }

    #[test]
    fn test_builder_method_chaining_comprehensive() {
        let config = PasswordConfig::new()
            .with_length(24)
            .with_uppercase(false)
            .with_lowercase(true)
            .with_digits(true)
            .with_symbols(false)
            .with_exclude_ambiguous(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 24);

        // Should only have lowercase letters and digits
        assert!(password.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(!password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(!password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)));
    }

    #[test]
    fn test_builder_method_overriding() {
        let config = PasswordConfig::new()
            .with_length(10)
            .with_length(20) // Override previous setting
            .with_symbols(true)
            .with_symbols(false); // Override previous setting

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 20); // Should use the last length setting
        assert!(!password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c))); // Should not have symbols
    }

    #[test]
    fn test_specific_ambiguous_character_exclusion() {
        let config = PasswordConfig::new()
            .with_length(200) // Large sample
            .with_exclude_ambiguous(true);

        let password = config.generate().unwrap();

        // Test specific ambiguous characters are excluded
        let ambiguous_chars = ['0', 'O', '1', 'l', 'I'];
        for ch in ambiguous_chars {
            assert!(
                !password.contains(ch),
                "Password should not contain ambiguous character '{ch}'"
            );
        }
    }

    #[test]
    fn test_specific_ambiguous_character_inclusion() {
        let config = PasswordConfig::new()
            .with_length(500) // Very large sample to ensure we get ambiguous chars
            .with_exclude_ambiguous(false);

        let password = config.generate().unwrap();

        // With a very large sample, we should get at least some ambiguous characters
        let ambiguous_chars = ['0', 'O', '1', 'l', 'I'];
        let has_any_ambiguous = ambiguous_chars.iter().any(|&ch| password.contains(ch));
        assert!(
            has_any_ambiguous,
            "Large password should contain at least some ambiguous characters"
        );
    }

    #[test]
    fn test_character_set_exact_contents() {
        // Test that we get exactly the expected characters for each set
        let test_cases = [
            // (config, expected_chars, forbidden_chars)
            (
                PasswordConfig::new()
                    .with_uppercase(true)
                    .with_lowercase(false)
                    .with_digits(false)
                    .with_symbols(false)
                    .with_exclude_ambiguous(true),
                "ABCDEFGHJKLMNPQRSTUVWXYZ", // Excludes I, O
                "IO0123456789!@#$%^&*()_+-=[]{}|;:,.<>?abcdefghijklmnopqrstuvwxyz",
            ),
            (
                PasswordConfig::new()
                    .with_uppercase(false)
                    .with_lowercase(true)
                    .with_digits(false)
                    .with_symbols(false)
                    .with_exclude_ambiguous(true),
                "abcdefghijkmnopqrstuvwxyz", // Excludes l only
                "lIO0123456789!@#$%^&*()_+-=[]{}|;:,.<>?ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            ),
            (
                PasswordConfig::new()
                    .with_uppercase(false)
                    .with_lowercase(false)
                    .with_digits(true)
                    .with_symbols(false)
                    .with_exclude_ambiguous(true),
                "23456789", // Excludes 0, 1
                "01lIOi!@#$%^&*()_+-=[]{}|;:,.<>?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            ),
            (
                PasswordConfig::new()
                    .with_uppercase(false)
                    .with_lowercase(false)
                    .with_digits(false)
                    .with_symbols(true)
                    .with_exclude_ambiguous(true),
                "!@#$%^&*+-=?", // Reduced symbol set
                "()_[]{}|;:,.<>lIO0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
            ),
        ];

        for (config, expected_chars, forbidden_chars) in test_cases {
            let password = config.generate().unwrap();

            // Check that all characters in the password are from the expected set
            for ch in password.chars() {
                assert!(
                    expected_chars.contains(ch),
                    "Character '{ch}' should be in expected set '{expected_chars}'"
                );
                assert!(
                    !forbidden_chars.contains(ch),
                    "Character '{ch}' should not be in forbidden set"
                );
            }
        }
    }

    #[test]
    fn test_minimum_character_requirements() {
        // Test that passwords contain at least one character from each enabled set
        let config = PasswordConfig::new()
            .with_length(20)
            .with_uppercase(true)
            .with_lowercase(true)
            .with_digits(true)
            .with_symbols(true);

        // Test multiple generations to ensure consistency
        for _ in 0..10 {
            let password = config.generate().unwrap();

            assert!(
                password.chars().any(|c| c.is_ascii_uppercase()),
                "Password should contain at least one uppercase letter"
            );
            assert!(
                password.chars().any(|c| c.is_ascii_lowercase()),
                "Password should contain at least one lowercase letter"
            );
            assert!(
                password.chars().any(|c| c.is_ascii_digit()),
                "Password should contain at least one digit"
            );
            assert!(
                password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)),
                "Password should contain at least one symbol"
            );
        }
    }

    #[test]
    fn test_memorable_password_syllable_count() {
        let password = generate_memorable_password();

        // Remove digits to get just the syllable part
        let letters_only: String = password.chars().filter(char::is_ascii_alphabetic).collect();

        // Based on implementation: 2 words × 3-4 syllables × 2 chars per syllable = 12-16 chars
        assert!(
            letters_only.len() >= 12,
            "Should have at least 12 letters from syllables, got {}",
            letters_only.len()
        );
        assert!(
            letters_only.len() <= 16,
            "Should have at most 16 letters from syllables, got {}",
            letters_only.len()
        );
    }

    #[test]
    fn test_memorable_password_digit_count() {
        let password = generate_memorable_password();
        let digit_count = password.chars().filter(char::is_ascii_digit).count();

        // Based on implementation: 1 separator digit + 2 ending digits = 3 total
        assert_eq!(digit_count, 3, "Memorable password should have exactly 3 digits");
    }

    #[test]
    fn test_memorable_password_capitalization_pattern() {
        for _ in 0..10 {
            let password = generate_memorable_password();

            // First character should be uppercase
            assert!(
                password.chars().next().unwrap().is_ascii_uppercase(),
                "First character should be uppercase"
            );

            // Find the first digit (separator)
            if let Some(separator_pos) = password.chars().position(|c| c.is_ascii_digit()) {
                let chars: Vec<char> = password.chars().collect();

                let mut first_syllable_chars = 0;
                for (i, &ch) in chars.iter().enumerate() {
                    if i >= separator_pos {
                        break;
                    }
                    if ch.is_ascii_alphabetic() {
                        first_syllable_chars += 1;
                        if first_syllable_chars <= 2 {
                            assert!(
                                ch.is_ascii_uppercase(),
                                "Character '{ch}' at position {i} in first syllable should be uppercase"
                            );
                        } else {
                            assert!(
                                ch.is_ascii_lowercase(),
                                "Character '{ch}' at position {i} should be lowercase (after first syllable)"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_memorable_password_structure_consistency() {
        // Test the general structure: Word1 + Digit + Word2 + DigitDigit
        for _ in 0..20 {
            let password = generate_memorable_password();
            let chars: Vec<char> = password.chars().collect();

            // Should start with uppercase letter
            assert!(chars[0].is_ascii_uppercase());

            // Should end with two digits
            let len = chars.len();
            assert!(chars[len - 1].is_ascii_digit(), "Should end with digit");
            assert!(chars[len - 2].is_ascii_digit(), "Second to last should be digit");

            // Should have exactly 3 digits total
            let digit_count = password.chars().filter(char::is_ascii_digit).count();
            assert_eq!(digit_count, 3, "Should have exactly 3 digits");
        }
    }

    #[test]
    fn test_performance_large_passwords() {
        use std::time::Instant;

        let start = Instant::now();
        let config = PasswordConfig::new().with_length(128);

        for _ in 0..100 {
            let password = config.generate().unwrap();
            assert_eq!(password.len(), 128);
        }

        let duration = start.elapsed();
        assert!(
            duration.as_secs() < 5,
            "Password generation should complete within 5 seconds"
        );
    }

    #[test]
    fn test_character_distribution_balance() {
        let config = PasswordConfig::new().with_length(400); // Large sample
        let password = config.generate().unwrap();

        let uppercase_count = password.chars().filter(char::is_ascii_uppercase).count();
        let lowercase_count = password.chars().filter(char::is_ascii_lowercase).count();
        let digit_count = password.chars().filter(char::is_ascii_digit).count();
        let symbol_count = password
            .chars()
            .filter(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(*c))
            .count();

        // With a large sample, each character type should appear reasonably frequently
        // Allow for some variance but ensure no type is completely absent or overwhelming
        assert!(
            uppercase_count >= 20,
            "Should have at least 20 uppercase chars, got {uppercase_count}"
        );
        assert!(
            lowercase_count >= 20,
            "Should have at least 20 lowercase chars, got {lowercase_count}"
        );
        assert!(digit_count >= 5, "Should have at least 5 digits, got {digit_count}");
        assert!(symbol_count >= 5, "Should have at least 5 symbols, got {symbol_count}");

        // No single type should dominate (more than 80% of total)
        assert!(uppercase_count < 320, "Uppercase shouldn't dominate");
        assert!(lowercase_count < 320, "Lowercase shouldn't dominate");
        assert!(digit_count < 320, "Digits shouldn't dominate");
        assert!(symbol_count < 320, "Symbols shouldn't dominate");
    }

    #[test]
    fn test_all_builder_methods_together() {
        let config = PasswordConfig::new()
            .with_length(32)
            .with_uppercase(true)
            .with_lowercase(true)
            .with_digits(true)
            .with_symbols(true)
            .with_exclude_ambiguous(true);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 32);
        assert_eq!(config.length, 32);
        assert!(config.include_uppercase);
        assert!(config.include_lowercase);
        assert!(config.include_digits);
        assert!(config.include_symbols);
        assert!(config.exclude_ambiguous);

        // Verify the password follows the configuration
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| "!@#$%^&*+-=?".contains(c))); // Reduced symbol set

        // Should not contain ambiguous characters
        let ambiguous = "0O1lI";
        assert!(!password.chars().any(|c| ambiguous.contains(c)));
    }

    #[test]
    fn test_password_entropy_distribution() {
        // Test that repeated password generation produces varied results
        let config = PasswordConfig::new().with_length(16);
        let mut passwords = std::collections::HashSet::new();

        // Generate many passwords
        for _ in 0..100 {
            let password = config.generate().unwrap();
            passwords.insert(password);
        }

        // Should have generated mostly unique passwords (very high probability)
        assert!(
            passwords.len() >= 95,
            "Should generate mostly unique passwords, got {} unique out of 100",
            passwords.len()
        );
    }

    #[test]
    fn test_edge_case_single_character_type_minimum_length() {
        let config = PasswordConfig::new()
            .with_length(4) // Minimum length
            .with_uppercase(true)
            .with_lowercase(false)
            .with_digits(false)
            .with_symbols(false);

        let password = config.generate().unwrap();
        assert_eq!(password.len(), 4);
        assert!(password.chars().all(|c| c.is_ascii_uppercase()));
        assert!(password.chars().any(|c| c.is_ascii_uppercase())); // At least one char
    }
}
