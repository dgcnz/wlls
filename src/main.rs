use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser};
use unicode_normalization::UnicodeNormalization;

use wlls::{collect_references, lookup_filename_in_vault, vault_contents, WalkOptions};

#[derive(Parser, Debug)]
#[command(name = "wlls", about = "List Obsidian wiki-linked files", version)]
struct Cli {
    /// Recurse through linked markdown notes
    #[arg(short = 'R', long = "recursive", action = ArgAction::SetTrue)]
    recursive: bool,
    /// Skip unresolved references instead of failing
    #[arg(long = "skip-missing-refs", action = ArgAction::SetTrue)]
    skip_missing_refs: bool,
    /// Path to the vault root
    vault_root: PathBuf,
    /// One or more note paths (absolute or vault-relative)
    notes: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.notes.is_empty() {
        return Err(anyhow!("At least one note path is required"));
    }

    let vault_root = cli
        .vault_root
        .canonicalize()
        .context("vault_root does not exist")?;
    if !vault_root.is_dir() {
        return Err(anyhow!("vault_root is not a directory: {}", vault_root.display()));
    }

    let vault_files = vault_contents(&vault_root, WalkOptions::default())
        .context("failed to enumerate vault contents")?;

    let mut queue = VecDeque::new();
    let mut outputs = HashSet::new();
    let mut visited = HashSet::new();
    for note in &cli.notes {
        let resolved = resolve_input_note(note, &vault_root, &vault_files)
            .with_context(|| format!("invalid input note: {}", note.display()))?;
        outputs.insert(resolved.clone());
        queue.push_back(resolved);
    }

    while let Some(note) = queue.pop_front() {
        if !visited.insert(note.clone()) {
            continue;
        }
        let content = fs::read_to_string(&note)
            .with_context(|| format!("failed to read note {}", note.display()))?;
        let refs = collect_references(&content);

        for raw_ref in refs {
            let Some(target) = resolve_reference(&raw_ref, &vault_files) else {
                if cli.skip_missing_refs {
                    eprintln!(
                        "warning: skipping unresolved reference '{}' from {}",
                        raw_ref,
                        note.display()
                    );
                    continue;
                }
                return Err(anyhow!(
                    "could not resolve reference '{}' from {}",
                    raw_ref,
                    note.display()
                ));
            };
            let target = target
                .canonicalize()
                .with_context(|| format!("failed to canonicalize {}", target.display()))?;
            outputs.insert(target.clone());

            if cli.recursive && is_markdown(&target) {
                queue.push_back(target);
            }
        }
    }

    let mut sorted: Vec<_> = outputs.into_iter().collect();
    sorted.sort();
    for path in sorted {
        println!("{}", path.display());
    }

    Ok(())
}

fn resolve_input_note(
    note: &Path,
    vault_root: &Path,
    vault_files: &[PathBuf],
) -> Result<PathBuf> {
    let path = if note.is_absolute() {
        note.to_path_buf()
    } else {
        vault_root.join(note)
    };
    let canonical = path
        .canonicalize()
        .with_context(|| format!("note path does not exist: {}", path.display()))?;

    if !canonical.starts_with(vault_root) {
        return Err(anyhow!(
            "note is outside vault_root: {}",
            canonical.display()
        ));
    }
    if !vault_files.iter().any(|p| same_file(p, &canonical)) {
        return Err(anyhow!(
            "note not found in vault scan: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn same_file(a: &Path, b: &Path) -> bool {
    normalize_path(a) == normalize_path(b)
}

fn resolve_reference(reference: &str, vault_contents: &[PathBuf]) -> Option<PathBuf> {
    lookup_filename_in_vault(reference, vault_contents).cloned()
}

fn is_markdown(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("md"))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().nfc().collect::<String>()
}
