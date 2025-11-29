# wlls

<p align="center">
  <img src="assets/logo.png" width="200" alt="Logo">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/release-v0.1.1-green" alt="Release"/>
  <img src="https://img.shields.io/badge/license-BSD--2--Clause--Patent-blue" alt="License"/>
</p>

<p align="center">
  <strong>List Obsidian wikilinks recursively</strong><br>
</p>


**Usage**

```sh
cargo install wlls
wlls -R /path/to/vault Root/Note.md Root/OtherNote.md
```

- Input paths can be vault-relative or absolute (under the vault).
- `-R/--recursive` follows markdown links recursively; without it, only direct references are listed.
- `--skip-missing-refs` warns and continues when a reference cannot be resolved; by default the command fails on the first missing reference.
- Output is absolute paths, one per line.

**Note**

We reuse [`obsidian-export`](https://github.com/zoni/obsidian-export) parsing/resolution and walker code verbatim; custom logic is confined to `src/main.rs` to avoid divergence and reimplementation.
