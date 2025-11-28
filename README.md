# wlls

A minimal CLI to (recursively) list Obsidian wikilink targets, using `obsidian-export` parsing/resolution and walker code. We intentionally keep our own logic limited to `src/main.rs`; all other modules are copied verbatim from obsidian-export to reduce divergence and avoid reimplementing parsing.
