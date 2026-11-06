# Pebble Launcher feature inventory

**Legend:** IMPLEMENTED IN SOURCE = code is present but not yet runtime-tested; PLANNED = no complete implementation. No product release is authorized.

| Area | Current status | Details |
|---|---|---|
| Lightweight dark desktop UI | Implemented in source | Native Tauri shell with framework-free UI |
| Microsoft account profiles | Implemented in source | Multi-account device code, official service exchange, keyring |
| Offline profiles | Implemented in source | Locally generated offline UUID, offline-only |
| Session-token import | Implemented in source | Validates official profile, keyring, no refresh |
| Account switching | Implemented in source | Selected account used for subsequent launches |
| Minecraft Vanilla installation | Implemented in source | Modern versions with metadata `arguments` |
| Snapshot selection | Implemented in source | Version manifest filter |
| Java launch and RAM | Implemented in source | Manual Java path, process logs |
| Named isolated instances | Implemented in source | Separate local game directories |
| Local mod/world inspection | Implemented in source | Read folders and open file manager |
| Fabric | Planned | Metadata/installer resolution and tests |
| Forge / NeoForge / Quilt | Planned | Loader installers, libraries, compatibility |
| Automatic Java installation | Planned | Runtime download, hash/signature verification |
| Modrinth / CurseForge | Planned | Search, download and dependency solver |
| Modpacks | Planned | Modrinth / CurseForge / FTB / ATLauncher |
| World backups and repair | Planned | Backups, verification and restoration |
| Server manager | Planned | Local servers, plugins, config, console |
| Friends / multiplayer / voice | Planned | Separate optional service and game components |
| Skins / capes / cosmetics | Planned | Profile and API integration |
| CLI / automation | Planned | Rust library boundaries first |
| In-game HUD / replays / controller | Planned | Requires version-matched client-side mod |
| Bedrock (Windows) | Planned | Platform-specific; not in Linux AppImage |
| Other account services | Not planned | Only Microsoft, offline and user-owned session import |

## Proposed architecture for subsequent integrations

1. Stabilize Mojang metadata parsing, authentication, Java launching and dedicated named instances.
2. Extract the Rust launcher engine into a library crate and add reliable mod-loader installation.
3. Add mod/modpack dependencies and Java runtime management.
4. Add world/server management, repairs and diagnostics.
5. Implement opt-in multiplayer features and the separate version-specific Minecraft-side client mod.
6. Test Linux AppImage first, Windows later. Keep the interface compact by lazy-loading feature pages.

## Third-party licensing

- Lucerna: GPL-3.0-or-later
- Arctic Launcher: GPL-3.0-or-later
- OpenLauncher: GPL-2.0 as stated by the uploaded source archive (verify exact license terms before reusing code)
- Meteor Client: GPL-3.0

Do **not** combine copied GPL-2.0-only implementation code with GPL-3.0 source in a single distribution without proper license review or permission. This snapshot uses an independently authored implementation and its original MIT baseline.
