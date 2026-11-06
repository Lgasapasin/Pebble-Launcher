# Pebble Launcher — V0.2 source development snapshot

**A lightweight Minecraft Java Edition desktop launcher for Linux and Windows.**
Built with **Tauri 2**, **Rust**, and **framework-free TypeScript/CSS**.

> **Source-only development snapshot, not a stable release.** This package was prepared privately and has not been published, compiled on CI, or tested against live Minecraft accounts. The owner's release hold remains in effect. Do **not** mistake a successful UI typecheck for a working native app.

## What's in this source right now

- Dark purple accounts page styled after the approved design, with a minimal left sidebar, lists, cards, profile details, and responsive account actions.
- Multiple Microsoft accounts (device-code sign-in), multiple local/offline profiles, and **session-token import** for accounts you own.
- Session-token import validates against Minecraft's **official profile service**; only account metadata is saved in `accounts.json`. Credentials require the OS keyring/credential manager; imported tokens **cannot be silently refreshed**.
- Account selection, remove, validate and active-account switching for **future game launches**.
- Mojang Vanilla release/snapshot manifest, SHA-1-checked client/library/asset downloads, installation progress and repair.
- Named local instances with separate game directories; create/list/remove, open instance folders and start vanilla Minecraft in a selected instance.
- List local mod JARs and world folders from a selected instance. Actual mod-loader installation is **not** provided in this version.
- Configurable Java executable, RAM, Microsoft application ID; Minecraft process logs (redacting Minecraft access tokens in captured logs).
- Browser-only **UI preview**: `npm run web:dev`, no native backend needed. Preview displays obviously fictitious sample accounts and cannot authenticate or launch.
- Manual-only **Linux x86-64 AppImage** workflow. No automated publish/release steps.

### Still to be implemented

The requested long-term scope from **Lucerna**, **OpenLauncher**, and **Arctic Launcher** is much larger than this development snapshot. The following are **planned, not finished**:

- Full Fabric/Forge/NeoForge/Quilt installation and metadata resolution; Java auto-download.
- Modrinth/CurseForge installs, updates, dependency solver and supported loaders.
- Modpack support (Modrinth, CurseForge, FTB and ATLauncher).
- World backups, server creation/management, crash diagnostics/repair and automatic updates.
- Skins, capes, friends, multiplayer sharing/voice and in-game client features.
- Windows Bedrock handling, controller, replay, HUD, and CLI tools.

The **Mods / Modpacks / Servers** UI intentionally makes missing capabilities explicit instead of providing buttons that pretend to work.

## Source structure

```text
src/main.ts                     UI, navigation, account/instance workflows
src/style.css                   Dark theme (no component framework)
src/assets/landscape.webp      Cropped background from approved generated UI art
src-tauri/src/auth.rs           Multi-account registry + Microsoft login + token vault
src-tauri/src/instances.rs      Named instance management and local file browsing
src-tauri/src/minecraft.rs      Vanilla installation, native extraction, Java launch
src-tauri/src/models.rs         Minecraft manifest types
src-tauri/src/net.rs            HTTPS download validation, file integrity
src-tauri/src/storage.rs        App-data paths and path traversal validation
src-tauri/src/lib.rs            Tauri commands
.github/workflows/build.yml     Manual AppImage workflow only
scripts/check_source.py         Source integrity/delimiter checks
FEATURES.md                     Integration roadmap and real status
```

## Preview the interface without Rust

With **Node 22+** and npm:

```bash
npm install
npm run check
npm run web:dev
```

Open the Vite URL in a browser. **This is a UI-only preview.** Sample users are not stored or authenticated; any game/account action explains that a native build is needed.

## Build an AppImage on GitHub (when you choose)

The `.github/workflows/build.yml` workflow is **manual (`workflow_dispatch`) only**. It does not automatically publish a release or push to a registry. See `BUILD-ON-GITHUB.md`.

This source archive has **not** been uploaded to GitHub or built.

### Native development prerequisites

- Node 22+ and npm
- Rust stable + Cargo
- Linux: Tauri 2 prerequisites including GTK/WebKitGTK 4.1, OpenSSL, librsvg and libayatana-appindicator
- Java installed separately, matching the Minecraft version's `javaVersion.majorVersion`

See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run check
python3 scripts/check_source.py
npm run dev
```

For a **local Linux AppImage** after confirming native compilation:

```bash
npm run build -- --bundles appimage
```

## Microsoft login configuration

In **Settings**, enter an **approved Microsoft public-client application ID** that supports the device-code flow and has the permissions required for the Minecraft service. New registrations may require additional Minecraft API approval. The launcher cannot bypass those permissions, and no working publisher ID is provided.

From **Accounts**, click **Microsoft**, sign in in your browser through the official device flow, and then select the account. A valid owned Minecraft Java Edition profile is required for authenticated game play.

**Session-token import** is for your own existing Minecraft Java Edition access token only. The launcher sends it only to Minecraft's official profile service to validate identity, then places the token in your OS credential vault. Imported sessions expire and require re-import. The launcher never prints imported tokens or saves them in app settings.

**Offline profiles** are for local/offline game play; they do not verify ownership or enable official online-mode servers.

### Security notes

- Never provide a Microsoft password to Pebble Launcher.
- Do not send session tokens to another person or paste them into issue trackers, screenshots, or logs.
- A working OS keyring is **required** for stored Microsoft and imported-session credentials; on Linux, your desktop keyring/Secret Service should be available. The backend refuses insecure plaintext fallback.
- Game access tokens are passed to Java as Minecraft arguments, so they may still be visible to OS process inspection while the game is running. Captured logs redact token text.
- Removing an instance deletes its local game directory **including world saves**; the UI requires confirmation. Back up worlds first.
- This independent launcher is not endorsed by Mojang or Microsoft. Respect Minecraft account and software terms.

## Source validation and caveats

- TypeScript passed `tsc --strict --noEmit` using a local type stub.
- Rust files received structural static checks, **but no Cargo compilation** has run here (Rust toolchain unavailable in this environment).
- No live account, download, game-launch, or AppImage packaging smoke test has been run.
- Old Minecraft version formats prior to the modern `arguments` metadata are not supported. Native extraction, OS rules and runtime compatibility need real-world testing.

## References and licensing

The source is an independent implementation that **does not copy/merge GPL code** from Lucerna, Arctic Launcher, OpenLauncher or Meteor Client. Those projects serve as architectural and feature references. The current Pebble project retains its MIT LICENSE. Before copying third-party code, verify GPL compatibility and preserve the relevant copyright/license notices.
