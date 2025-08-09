# Contributing to Chamber 🤝

Thank you for your interest in contributing to Chamber! We welcome contributions from everyone, whether you're fixing bugs, adding features, improving documentation, or helping with testing.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Development Guidelines](#development-guidelines)
- [Testing](#testing)
- [Documentation](#documentation)
- [Security Considerations](#security-considerations)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

## 📜 Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have the following installed:

- **Rust 1.89.0 or newer**: Install via [rustup](https://rustup.rs/)
- **Git**: For version control
- **A modern terminal**: For testing the TUI components

### Development Dependencies

Install additional development tools:

```bash
# Essential tools
cargo install cargo-nextest      # Better test runner
cargo install cargo-watch       # File watching
cargo install cargo-tarpaulin   # Code coverage
cargo install cargo-audit       # Security auditing
cargo install cargo-deny        # License and dependency checking

# Optional but recommended
cargo install cargo-machete     # Find unused dependencies
cargo install cargo-outdated    # Check for outdated dependencies
cargo install flamegraph        # Performance profiling
```
```
## 🛠️ Development Setup
1. **Fork and clone the repository**:
``` bash
   git clone https://github.com/your-username/chamber.git
   cd chamber
```
1. **Set up the development environment**:
``` bash
   # Check that everything compiles
   cargo check --all-targets --all-features
   
   # Run the test suite
   cargo test
   
   # Format code
   cargo fmt
   
   # Run linter
   cargo clippy --all-targets --all-features -- -D warnings
```
1. **Verify the setup**:
``` bash
   # Build and test the project
   cargo build
   cargo test --all-features
   
   # Try running Chamber
   cargo run -- --help
```
## 🤝 How to Contribute
### Ways to Contribute
- 🐛 **Bug Reports**: Found a bug? Please report it!
- 💡 **Feature Requests**: Have an idea? We'd love to hear it!
- 🔧 **Bug Fixes**: Fix bugs and submit pull requests
- ✨ **New Features**: Implement new functionality
- 📚 **Documentation**: Improve docs, add examples
- 🧪 **Testing**: Add tests, improve test coverage
- 🎨 **UI/UX**: Improve the terminal interface
- 🔐 **Security**: Security reviews and improvements

### Finding Work
- Check the [Issues](https://github.com/your-org/chamber/issues) page
- Look for issues labeled `good first issue` or `help wanted`
- Check the [project roadmap](https://github.com/your-org/chamber/projects)
- Improve documentation or add examples

## 📐 Development Guidelines
### Code Style
We follow standard Rust conventions:
``` bash
# Format your code
cargo fmt

# Check for common mistakes
cargo clippy --all-targets --all-features -- -D warnings

# Check for typos
typos

# Ensure documentation builds
cargo doc --no-deps --all-features
```
### Coding Standards
- **Error Handling**: Use for application errors, specific error types for library code `anyhow`
- **Documentation**: All public APIs must have documentation
- **Testing**: New features require tests
- **Security**: Security-sensitive code needs extra attention and review
- **Performance**: Consider performance implications, especially for crypto operations

### Project Structure
``` 
chamber/
├── crates/
│   ├── vault/           # Core vault logic and cryptography
│   ├── cli/             # Command-line interface
│   ├── tui/             # Terminal user interface
│   └── import-export/   # Data serialization and migration
├── src/                 # Main binary and application logic
├── tests/              # Integration tests
├── benches/            # Performance benchmarks
├── docs/               # Additional documentation
└── examples/           # Usage examples
```
### Commit Messages
Use conventional commit format:
``` 
type(scope): description

[optional body]

[optional footer]
```
**Types**: `feat`, , `docs`, `test`, `refactor`, `perf`, `chore`, `ci` `fix`
**Examples**:
``` 
feat(vault): add master password rotation
fix(cli): handle empty vault gracefully
docs: update installation instructions
test(crypto): add ChaCha20 test vectors
```
## 🧪 Testing
### Running Tests
``` bash
# Run all tests
cargo test

# Run with nextest (faster, better output)
cargo nextest run

# Run specific test suite
cargo test --package chamber-vault

# Run integration tests
cargo test --test integration_tests

# Run with coverage
cargo tarpaulin --out html
```
### Test Categories
1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test component interactions
3. **Crypto Tests**: Verify cryptographic implementations
4. **CLI Tests**: Test command-line interface behavior
5. **Database Tests**: Test SQLite operations and migrations

### Writing Tests
- Add unit tests in the same file as the code (using `#[cfg(test)]`)
- Add integration tests in the `tests/` directory
- Use descriptive test names: `test_vault_creation_with_strong_password`
- Test both success and error cases
- For crypto code, include test vectors when possible

### Benchmarks
``` bash
# Run benchmarks
cargo bench

# Profile with flamegraph
cargo flamegraph --bench crypto_bench
```
## 📚 Documentation
### Documentation Standards
- **Public APIs**: Must have rustdoc comments
- **Examples**: Include usage examples in doc comments
- **README**: Keep README.md up to date
- **Architecture**: Document design decisions

### Building Documentation
``` bash
# Build documentation
cargo doc --no-deps --all-features --open

# Check for broken links
cargo doc --no-deps --all-features 2>&1 | grep warning
```
## 🔐 Security Considerations
Chamber is a security-critical application. Please keep these guidelines in mind:
### Security Review Process
1. **Crypto Changes**: All cryptographic code changes require thorough review
2. **Memory Safety**: Be mindful of sensitive data in memory
3. **Dependencies**: New dependencies require security review
4. **Input Validation**: Validate all user input rigorously

### Security Testing
``` bash
# Audit dependencies
cargo audit

# Check for security advisories
cargo deny check advisories

# Run security-focused tests
cargo test --features security-tests
```
### Reporting Security Issues
🚨 **Do not report security vulnerabilities through public GitHub issues.**
Instead, please email security issues to: [security@chamber-project.org]()
Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## 🔄 Pull Request Process
### Before Submitting
1. **Create an issue** (for non-trivial changes)
2. **Fork the repository**
3. **Create a feature branch**: `git checkout -b feature/my-feature`
4. **Make your changes**
5. **Add tests** for new functionality
6. **Update documentation** if needed

### Pre-submission Checklist
- Code compiles without warnings: `cargo build`
- All tests pass: `cargo test`
- Code is formatted: `cargo fmt`
- No clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- Documentation builds: `cargo doc --no-deps`
- Security audit passes: `cargo audit`
- Commit messages follow conventions

### Submission Process
1. **Push your branch**: `git push origin feature/my-feature`
2. **Create a Pull Request** through GitHub
3. **Fill out the PR template** completely
4. **Wait for review** and address feedback
5. **Merge** (will be done by maintainers)

### PR Template
``` markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Tests added/updated
- [ ] All tests pass
- [ ] Manual testing completed

## Security
- [ ] No new security vulnerabilities introduced
- [ ] Security-sensitive changes reviewed

## Documentation
- [ ] Documentation updated
- [ ] Examples added/updated
```
## 🎯 Review Process
### What We Look For
- **Correctness**: Does the code work as intended?
- **Security**: Are there any security implications?
- **Performance**: Is the performance impact acceptable?
- **Maintainability**: Is the code easy to understand and maintain?
- **Testing**: Are there adequate tests?
- **Documentation**: Is the code properly documented?

### Review Timeline
- **Initial Response**: Within 48 hours
- **Full Review**: Within 1 week for most PRs
- **Security Reviews**: May take longer due to additional scrutiny

## 📈 Development Workflow
### Recommended Workflow
``` bash
# Stay up to date
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/my-awesome-feature

# Make changes and test frequently
cargo watch -x "test --all-features"

# Before committing
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test

# Commit with conventional format
git commit -m "feat(vault): add password strength validation"

# Push and create PR
git push origin feature/my-awesome-feature
```
### Continuous Integration
Our CI pipeline runs:
- **Tests**: All test suites across multiple platforms
- **Linting**: Format and clippy checks
- **Security**: Dependency auditing
- **Coverage**: Code coverage analysis
- **Documentation**: Doc generation and link checking

## 🌍 Community
### Getting Help
- **GitHub Discussions**: For questions and general discussion
- **Issues**: For bug reports and feature requests
- **Discord/Matrix**: Real-time chat (links in README)

### Code of Conduct
We are committed to providing a welcoming and inclusive environment. Please read our [Code of Conduct](CODE_OF_CONDUCT.md).
### Recognition
Contributors are recognized in:
- : Active contributors list **README.md**
- **Releases**: Changelog mentions
- **All Contributors**: Comprehensive contributor recognition

## 📞 Getting in Touch
- **Maintainers**: [@mikeleppane](https://github.com/mikeleppane)
- **General Questions**: GitHub Discussions
- **Security Issues**: security@chamber-project.org

## 📝 License
By contributing to Chamber, you agree that your contributions will be licensed under the [MIT License](LICENSE).
Thank you for contributing to Chamber! Your help makes this project better for everyone. 🙏
