# Global Coding Conventions

## General

- Do not write a comment without justification. If code is self-explanatory, leave it uncommented.
- Do not create small helper functions that are referenced only once. Inline the logic.
- Prefer editing existing files over creating new ones when the change is small.

## Git

- All commits must be in Conventional Commits style and in English.

## Rust

### Linting

- Use `cargo clippy` to check files.

### Module Organization

- Prefer private modules (`mod foo;`) with explicitly re-exported public API via `pub use` in `mod.rs`.
- Keep internal helpers and types private; only export what the crate consumer needs.
- Use `mod.rs` barrel files to control the public surface of each module.

### Module Size

- Target modules under 500 LoC (excluding tests).
- If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending the existing file unless there is a strong documented reason not to.
- When extracting code from a large module, move the related tests and module/type docs toward the new implementation so the invariants stay close to the code that owns them.
- Avoid logic in `mod.rs` files, favor creating a new file for logic to keep `mod.rs` clean.

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
- Use `#[cfg(test)]` visibility (e.g. `pub` on test-only constructors like `open_in_memory`) rather than making internals permanently public.

### Types & Derives

- Derive only what is needed. Common set: `Debug`, `Clone`, `PartialEq`, `Eq`.
- Use builder-style methods (`with_*` returning `Self`) for optional configuration on structs.
- Use `clap` derive macros for CLI argument parsing.

### Imports

- Do not separate `use` groups with a blank line. All `use` statments should be in a single block. Let the formatter sort the groups automatically.

### Tests

- Place unit tests in a `#[cfg(test)] mod tests` block at the bottom of the same file.
- Prefer comparing whole-object equality (`assert_eq!`) over asserting individual fields one by one.
- Use `assert_matches!` (from `assert_matches` crate) for enum variant matching.
- Use `assert_fs` and `assert_cmd` for filesystem and CLI integration tests.
- Name tests using `when_<condition>_then_<action>_should_<expected>` convention.
- Structure tests in Arrange-Act-Assert (AAA) form. Do not add `// Arrange`, `// Act`, `// Assert` comments - use blank lines to separate the sections.

## Constraints

- Never commit proprietary Ubiquiti code or assets.
- Never commit logs or captures without permission from the User.
