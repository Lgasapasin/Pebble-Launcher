#!/usr/bin/env python3
"""Lightweight CI guard against the previously truncated Rust-file uploads.

This is NOT a Rust parser or compiler; cargo check is still mandatory.
"""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
required = {
    "src-tauri/src/minecraft.rs": ["pub async fn install_version(", "pub async fn launch_game(", "extract_natives(", "mod tests"],
    "src-tauri/src/auth.rs": ["pub async fn import_session(", "pub async fn complete_login(", "pub async fn access_for_launch(", "pub fn list_accounts("],
    "src-tauri/src/instances.rs": ["pub fn create_instance(", "pub fn list_instances(", "pub fn game_directory("],
    "src-tauri/src/lib.rs": ["mod auth;", "mod instances;", "generate_handler!"],
    "src/main.ts": ["accountsView", "import_session", "launch_game"],
    "src/style.css": [".accounts-columns", ".account-card", ".method-grid"],
    ".github/workflows/build.yml": ["workflow_dispatch:", "--bundles appimage"],
}
errors = []
for filename, tokens in required.items():
    file = root / filename
    if not file.exists():
        errors.append(f"Missing {filename}")
        continue
    source = file.read_text(encoding="utf-8")
    for token in tokens:
        if token not in source:
            errors.append(f"Missing required text {token!r} in {filename}")
    if filename.endswith(".rs") and source.count("{") < 1:
        errors.append(f"Suspiciously short Rust module: {filename}")
    if filename.endswith(".rs") and len(source.splitlines()) < 8:
        errors.append(f"Truncated Rust module: {filename}")
if errors:
    print("Source integrity check failed:")
    print("\n".join(" - " + e for e in errors))
    sys.exit(1)
print("Source integrity checklist passed. Cargo compilation is still required.")
