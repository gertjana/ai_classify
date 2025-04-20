# Contributing to Classify

Thank you for considering contributing to Classify! This document provides guidelines and instructions for contributing to this project.

## Getting Started

### Prerequisites

- Rust (stable channel)
- Redis server (for running integration tests)

### Setting Up Development Environment

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/classify.git
   cd classify
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

4. Run Redis integration tests (requires Redis server):
   ```bash
   cargo test -- --ignored
   ```

## Development Workflow

### Code Style

This project uses Rust's official code formatter `rustfmt` and linter `clippy`:

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy
```

### Running the Application

```bash
cargo run
```

### Making Changes

1. Create a new branch for your feature or bugfix
2. Make your changes
3. Run tests to ensure everything works
4. Create a pull request

## Pull Request Process

1. Ensure all tests pass and code is properly formatted
2. Update the README.md with details of changes if applicable
3. The pull request will be merged once it's reviewed and approved

## Continuous Integration

This project uses GitHub Actions for CI/CD:

- All tests are run on each push and pull request
- Code formatting and linting are checked
- Documentation is automatically generated and deployed

## Project Structure

- `src/storage/` - Storage implementations for content and tags
- `src/classifier/` - Classifier implementations for content
- `src/api/` - API endpoints and server
- `src/config/` - Configuration handling

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Run all tests including ignored ones
cargo test -- --ignored
```

## Documentation

To generate and view documentation:

```bash
cargo doc --open
```
