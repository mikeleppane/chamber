# Contributing to VisualVault

First off, thank you for considering contributing to VisualVault! It's people like you that make VisualVault
such a great tool. 🎉

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How Can I Contribute?](#how-can-i-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Enhancements](#suggesting-enhancements)
  - [Your First Code Contribution](#your-first-code-contribution)
  - [Pull Requests](#pull-requests)
- [Development Setup](#development-setup)
- [Style Guidelines](#style-guidelines)
  - [Git Commit Messages](#git-commit-messages)
  - [Rust Style Guide](#rust-style-guide)
  - [Documentation Style Guide](#documentation-style-guide)
- [Testing Guidelines](#testing-guidelines)
- [Project Structure](#project-structure)
- [Community](#community)

## Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct.
By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your feature or bugfix
4. Make your changes
5. Run tests and ensure they pass
6. Commit your changes
7. Push to your fork
8. Create a Pull Request

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check existing issues as you might find out that you don't need to create one.
When you are creating a bug report, please include as many details as possible:

**Bug Report Template:**

```markdown
**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Screenshots**
If applicable, add screenshots to help explain your problem.

**Environment:**
 - OS: [e.g. Ubuntu 22.04, macOS 13, Windows 11]
 - Rust version: [e.g. 1.85.0]
 - VisualVault version/commit: [e.g. 0.1.0 or commit hash]

**Additional context**
Add any other context about the problem here.
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

- **Use a clear and descriptive title**
- **Provide a step-by-step description** of the suggested enhancement
- **Provide specific examples** to demonstrate the steps
- **Describe the current behavior** and explain which behavior you expected to see instead
- **Explain why this enhancement would be useful** to most VisualVault users

### Your First Code Contribution

Unsure where to begin contributing? You can start by looking through these issues:

- Issues labeled `good first issue` - issues which should be relatively simple to implement
- Issues labeled `help wanted` - issues which need extra attention
- Issues labeled `documentation` - improvements or additions to documentation

### Pull Requests

1. **Fork and clone the repository**
2. **Create a new branch**: `git checkout -b feature/your-feature-name`
3. **Make your changes** and add tests for them
4. **Run the test suite**: `cargo test` and `cargo nextest run`
5. **Run clippy**: `cargo clippy -- -D warnings`
6. **Format your code**: `cargo fmt`
7. **Commit your changes**: Use a descriptive commit message
8. **Push to your fork**: `git push origin feature/your-feature-name`
9. **Submit a pull request**

## Development Setup

### Prerequisites

- Rust 1.85 or higher
- Git
- A terminal emulator with good Unicode support

### Building the Project

```bash
# Clone the repository
git clone https://github.com/yourusername/visualvault.git
cd visualvault

# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test core::scanner

# Run with nextest (recommended)
cargo nextest run

# Run with coverage
cargo tarpaulin --out Html
```

### Development Tools

We recommend installing these tools for a better development experience:

```bash
# Install development tools
cargo install cargo-watch    # Auto-rebuild on file changes
cargo install cargo-nextest  # Better test runner
cargo install cargo-tarpaulin # Code coverage
cargo install cargo-audit    # Security audit

# Watch for changes and run tests
cargo watch -x test

# Watch for changes and run the app
cargo watch -x run
```

## Style Guidelines

### Git Commit Messages

- Use the present tense ("Add feature" not "Added feature")
- Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit the first line to 72 characters or less
- Reference issues and pull requests liberally after the first line
- Use conventional commits format when possible:
  - `feat:` for new features
  - `fix:` for bug fixes
  - `docs:` for documentation changes
  - `style:` for formatting changes
  - `refactor:` for code refactoring
  - `test:` for adding tests
  - `chore:` for maintenance tasks

Examples:

```text

feat: add support for HEIC image format

- Add HEIC detection in media_types module
- Update scanner to handle HEIC files
- Add tests for HEIC file processing

Closes #123
```

### Rust Style Guide

We follow the standard Rust style guidelines:

- Run `cargo fmt` before committing
- Ensure `cargo clippy -- -D warnings` passes
- Use descriptive variable names
- Add documentation comments for public APIs
- Keep functions focused and small
- Use `Result<T, E>` for error handling
- Prefer `&str` over `String` for function parameters when possible

Example:

```rust
/// Organizes files based on the specified organization mode.
///
/// # Arguments
///
/// * `files` - Vector of files to organize
/// * `settings` - Configuration settings for organization
///
/// # Returns
///
/// Returns `Ok(OrganizationResult)` on success, or an error if organization fails.
///
/// # Example
///
/// ```
/// let result = organizer.organize_files(files, &settings).await?;
/// println!("Organized {} files", result.files_organized);
/// ```
pub async fn organize_files(
    &self,
    files: Vec<MediaFile>,
    settings: &Settings,
) -> Result<OrganizationResult> {
    // Implementation
}
```

### Documentation Style Guide

- Use triple-slash comments (`///`) for public items
- Include examples in documentation when helpful
- Document panic conditions with `# Panics`
- Document error conditions with `# Errors`
- Keep line length under 100 characters in documentation

## Testing Guidelines

### Writing Tests

- Write tests for all new functionality
- Place unit tests in the same file as the code they test
- Place integration tests in the `tests/` directory
- Use descriptive test names that explain what is being tested
- Use test fixtures and helper functions to reduce duplication

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper function for test setup
    async fn setup_test_environment() -> Result<(TempDir, Scanner)> {
        let temp_dir = TempDir::new()?;
        let scanner = Scanner::new();
        Ok((temp_dir, scanner))
    }

    #[tokio::test]
    async fn test_scanner_finds_jpeg_files() -> Result<()> {
        let (temp_dir, scanner) = setup_test_environment().await?;
        
        // Create test file
        let test_file = temp_dir.path().join("test.jpg");
        fs::write(&test_file, b"fake jpeg data").await?;
        
        // Run scanner
        let files = scanner.scan_directory(temp_dir.path(), false).await?;
        
        // Assertions
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].extension, "jpg");
        
        Ok(())
    }
}
```

### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test complete workflows
3. **UI Tests**: Test terminal UI components (when applicable)
4. **Performance Tests**: Benchmark critical paths

## Project Structure

```text
visualvault/
├── src/
│   ├── main.rs              # Application entry point
│   ├── app.rs               # Main application state and logic
│   ├── config/              # Configuration management
│   │   └── settings.rs      # Settings structure and defaults
│   ├── core/                # Core functionality
│   │   ├── scanner.rs       # File scanning logic
│   │   ├── organizer.rs     # File organization logic
│   │   ├── duplicate.rs     # Duplicate detection
│   │   └── file_cache.rs    # File metadata caching
│   ├── models/              # Data structures
│   │   ├── file_type.rs     # File type definitions
│   │   ├── media_file.rs    # Media file representation
│   │   └── filters.rs       # Filter definitions
│   ├── ui/                  # Terminal UI components
│   │   ├── dashboard.rs     # Dashboard view
│   │   ├── settings.rs      # Settings view
│   │   └── help.rs          # Help overlay
│   └── utils/               # Utility functions
│       ├── datetime.rs      # Date/time helpers
│       ├── format.rs        # Formatting utilities
│       └── media_types.rs   # Media type detection
├── tests/                   # Integration tests
├── Cargo.toml              # Project dependencies
└── README.md               # Project documentation
```

## Community

- **GitHub Issues**: For bug reports and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Pull Requests**: For code contributions

### Getting Help

If you need help, you can:

1. Check the [README](README.md) for usage information
2. Look through existing [issues](https://github.com/yourusername/visualvault/issues)
3. Create a new issue with the `question` label
4. Start a discussion in the [Discussions](https://github.com/yourusername/visualvault/discussions) section

## Recognition

Contributors who submit accepted pull requests will be added to the project's AUTHORS file and recognized
in the release notes.

Thank you for contributing to VisualVault! 🚀  
