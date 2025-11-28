# wlls

List Obsidian wikilink targets (recursively) for selective publishing

We reuse `obsidian-export` parsing/resolution and walker code verbatim; custom logic is confined to `src/main.rs` to avoid divergence and reimplementation.

Usage:

```sh
cargo install wlls
wlls -R /path/to/vault Root/Note.md Root/OtherNote.md
```

- Input paths can be vault-relative or absolute (under the vault).
- `-R/--recursive` follows markdown links recursively; without it, only direct references are listed.
- Output is absolute paths, one per line.
