# unifi-viewport-sdk

Goal: Implement a RUST SDK that can mimic a Ubiquiti Unifi Protect ViewPort that can be discovered, adopted, and
configured by a Protect console. For testing the ViewPort device SDK, a minimal controller implementation will also be
created.

The `spec/` folder contains the specification the SDK is being built against. The `docs/` folder should be updated to
reflect the changes made to the SDK.

### SDK Conventions

- Treat network data as untrusted. Validate data to the best of our knowledge. Don't silently hide issues, emit `warn!`
  for unexpected data to allow us to continuously learn about the protocols.
- As a reverse engineering project, we cannot guarantee our understanding of the protocol is correct. The SDK should
  give consuming code the flexibility to handle data we do not understand or that we have an incorrect interpretation
  of.

## General

- Do not write a comment without justification. If code is self-explanatory, leave it uncommented.
- Do not create small helper functions that are referenced only once. Inline the logic.
- Prefer editing existing files over creating new ones when the change is small.

## Git

- All commits must be in Conventional Commits style and in English.

## Rust

### Module Organization

- Prefer private modules (`mod foo;`) with explicitly re-exported public API via `pub use` in `mod.rs`.
- Keep internal helpers and types private; only export what the crate consumer needs.
- Use `mod.rs` barrel files to control the public surface of each module.

### Module Size

- Target modules under 500 LoC (excluding tests).
- If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending the existing file unless
  there is a strong, documented reason not to.
- When extracting code from a large module, move the related tests and module/type docs toward the new implementation so
  the invariants stay close to the code that owns them.
- Avoid logic in `mod.rs` files, favor creating a new file for logic to keep `mod.rs` clean.

### Logging

Use `tracing` for logging. Import the logging modules, e.g., `use tracing::info;` instead of using the absolute path,
e.g., `tracing::info!("Logging...");`.

Logging should always be in Sentence case (the first letter capitalized, proper names capitalized, using proper grammar,
etc.).

Logging Levels:

- `error`: For reporting errors that are not expected to occur during normal operation and typically require human
  intervention.
- `warn`: For reporting non-critical (recoverable) issues that may indicate a problem, but do not typically require
  human intervention.
- `info`: For reporting general information about the application's state or progress. Should be useful for an end-user
  to understand the application's behavior.
- `debug`: For detailed information that is primarily useful to developers.
- `trace`: For extremely detailed information required for low-level debugging.

### Error Handling

- Use `thiserror` for domain-specific error enums. Use `anyhow` for propagation in application code.
- Derive `Error` and `Debug` on error enums. Use `#[from]` for automatic conversions where appropriate.

### Match Statements

- Make match statements exhaustive. Avoid wildcard (`_`) arms unless the set of variants is genuinely open-ended.

### Comments

- Add short doc comments (`///`) to public `fn` and `struct` declarations. Keep them concise.
- Do not add doc comments to private internals unless the logic is non-obvious.
- Inline comments are only for non-obvious reasoning (e.g. `// SAFETY:` blocks).

### Visibility

- Default to private. Use `pub` only for the crate's public API surface.
- Use `#[cfg(test)]` visibility (e.g. `pub` on test-only constructors like `open_in_memory`) rather than making
  internals permanently public.

### Types & Derives

- Derive only what is needed. Common set: `Debug`, `Clone`, `PartialEq`, `Eq`.
- Use builder-style methods (`with_*` returning `Self`) for optional configuration on structs.
- Use `clap` derive macros for CLI argument parsing.

### Imports

- Do not separate `use` groups with a blank line. All `use` statments should be in a single block. Let the formatter
  sort the groups automatically.

```rust
// DO:
use std::io;
use thiserror::Error;
use crate::foo;

// NOT:
use std::io;

use thiserror::Error;
```

### Tests

- Place unit tests in a `#[cfg(test)] mod tests` block at the bottom of the same file.
- Prefer comparing whole-object equality (`assert_eq!`) over asserting individual fields one by one.
- Use `assert_matches!` (from `assert_matches` crate) for enum variant matching.
- Use `assert_fs` and `assert_cmd` for filesystem and CLI integration tests.
- Name tests using `when_<condition>_then_<action>_should_<expected>` convention.
- Structure tests in Arrange-Act-Assert (AAA) form. Do not add `// Arrange`, `// Act`, `// Assert` comments - use blank
  lines to separate the sections.
- Unit tests should be created for new code and updated when migrating old code.
- Integration tests should be created using the minimal controller code to verify the SDK's functionality.
- Tests should not rely on real hardware (which is why we are building a minimal controller).

### Verification

- Always use these commands to verify correctness:
    - `cargo fmt`
    - `cargo clippy`
    - `cargo test`
    - `cargo doc --no-deps`
    - `scripts/check_use_groups.sh`

## Constraints

- Never commit proprietary Ubiquiti code or assets.
- Never commit logs or captures without permission from the User.
