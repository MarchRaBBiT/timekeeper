---
name: Coding Guidelines
description: Enforces Rust coding standards, strict linting, and error handling patterns.
---

# Rust Coding Guidelines

## Default Project Settings
When modifying `Cargo.toml`, ensure these settings are present:

```toml
[package]
edition = "2021" # Or 2024 if available
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
```

## Code Style
- **Variables/Functions**: `snake_case`
- **Types/Traits**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Line Length**: Max 100 characters

## Error Handling
- **Avoid `unwrap()`**: Use `?` or explicit `match` in library code/handlers.
- **Context**: Use `anyhow::Context` or custom errors to provide context.

```rust
// Good
fn read_config() -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string("config.toml")?;
    toml::from_str(&content).map_err(Into::into)
}

// Bad
fn read_config() -> Config {
    toml::from_str(&std::fs::read_to_string("config.toml").unwrap()).unwrap()
}
```

## Common Fixes
- **E0382 (Used after move)**: Clone or borrow.
- **E0597 (Lifetime too short)**: Extend scope or use owned types.
- **E0502 (Borrow conflict)**: Keep scopes small, or use interior mutability (`RefCell`) if strictly necessary.
