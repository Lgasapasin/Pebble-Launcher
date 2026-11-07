# Pebble Launcher — v0.3 development SOURCE ONLY

A lightweight Minecraft Java Edition launcher built with **Tauri 2 + Rust + TypeScript**.

**PRIVATE SOURCE HANDOFF — NOT RELEASED / NOT A TESTED APPIMAGE.** This source is provided at the owner's request. Nothing has been published to GitHub or distributed as an installer. Native compilation and live Minecraft/Modrinth logins are not verified here.

## What changed since v0.2

### Imported Minecraft session tokens
- `keyring` now explicitly enables **Linux Secret Service** and **Windows Credential Manager** backends instead of an in-process mock.
- The Linux AppImage build workflow installs `libdbus-1-dev` and `pkg-config` for compilation.
- Import verifies the token with Minecraft's official profile endpoint, stores it in the OS credential vault, and **reads it back immediately** to detect credential-storage errors at import time. Token values are not saved in JSON.
- On Linux Mint, a running and unlocked Secret Service provider (such as GNOME Keyring) is still required. **Existing sessions stored with the old mock backend need re-importing.**

### Automatic Java selection
- `java.rs` probes configured Java, app-managed runtimes, `JAVA_HOME`, installed `/usr/lib/jvm` runtimes on Linux, and Java on PATH.
- Uses Mojang's `javaVersion.majorVersion` from the actual selected Minecraft version. Minecraft 1.21.x generally needs Java 21; newer game releases may require Java 25. Java 21 cannot safely be substituted for Java 25.
- When no matching Java is installed, can download a supported **Eclipse Temurin JRE** (x86-64 Linux/Windows), verify **SHA-256**, and unpack under Pebble's user data directory without administrator privileges.
- Settings accepts `auto` (recommended) or a custom Java executable. The chosen Java is not required to match a manually configured global version.
- Managed Java download and extraction still need live-platform testing.

### Mod-loader and Modrinth integration
- Instance records now track a `loader` and `loaderVersion` without affecting previously created instances (defaults to `vanilla`).
- **Fabric / Quilt:** metadata and launcher profile from official loader APIs. Loader libraries are resolved into Pebble's shared library cache and merged into game launch classpath/args.
- **Forge / NeoForge:** experimental bridge to official Java installers using explicit build versions. **May not work** for versions with advanced install-time processors or atypical profiles; needs native testing.
- **Modrinth:** search compatible projects by exact game version and loader, download SHA-1-verified JARs, follow required project dependencies (bounded), and enable/disable/remove local mod files.
- All mod files belong to their own instance's `mods/` folder.
- UI provides mod-loader installation controls in **Instances** and Modrinth search/mod controls in **Mods**.

**Not yet implemented:** CurseForge and FTB/ATLauncher downloads, modpack import/export, advanced mod conflict resolution, reliable Forge/NeoForge multi-version install/launch, server manager, in-game Meteor components. The full requested merger of three reference launchers is a longer project; this ZIP does not pretend otherwise.

## Private build validation

```bash
npm install
npm run check
python3 scripts/check_source.py
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

If you choose to build an AppImage for private testing, a **manual-only** workflow is in `.github/workflows/build.yml`; no automatic GitHub Release or publishing is configured. GitHub Actions currently targets Ubuntu 22.04 Linux x86-64.

## Account instructions

1. Open **Accounts** → **Session Token**, import an *existing token belonging to you*.
2. On Linux, ensure GNOME Keyring/Secret Service is running and unlocked.
3. If you used the earlier Pebble source, **re-import** the session after updating, as the prior mocked credential store was not persistent.
4. Select the account, then **Validate**. Imported access tokens expire and cannot be automatically refreshed.

Do not paste access tokens into chat messages, log files, GitHub issues, or screenshots.

## Loader instructions

1. Create an instance tied to a Mojang game version.
2. In **Instances**, choose Fabric / Quilt and click **Install Loader** (latest compatible stable loader is chosen when build is blank).
3. Forge/NeoForge require the **exact official loader build**. Forge format: `1.21.11-61.2.5`; NeoForge format varies by release.
4. Once configured, select the instance under **Mods**, search Modrinth, install a compatible mod, and launch that instance.
5. Back up `saves/` before experimenting with loaders and modpacks.

Game mod loaders and Java are distributed by their upstream projects, not bundled in this source. Account authentication uses official Minecraft services, and online play requires valid account ownership.

## Licensing

Source in this project is independently written; the three uploaded reference launchers were studied for features, **not copied into this archive**. The existing MIT license applies to this independently authored project; reuse of GPL source from other projects would require separate license review and attribution.
