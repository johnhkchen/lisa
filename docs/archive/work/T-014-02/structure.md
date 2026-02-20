# T-014-02 Structure: Verify cargo install and package metadata

## Files Modified

### `Cargo.toml` (workspace root)
Add `authors` to `[workspace.package]`:
```toml
authors = ["John Chen <john.hk.chen@gmail.com>"]
```

### `crates/lisa-cli/Cargo.toml`
Add workspace inheritance:
```toml
authors.workspace = true
```

### `crates/lisa-core/Cargo.toml`
Add workspace inheritance:
```toml
authors.workspace = true
```

## Files Unchanged

### `crates/lisa-plugin/Cargo.toml`
Already has `publish = false`. No changes needed — authors field not required for unpublished crates.

## No New Files

All changes are metadata additions to existing Cargo.toml files.
