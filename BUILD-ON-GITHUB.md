# AppImage-only build (manual, no release)

The source owner requested **no release until explicit authorization**. The workflow below only runs if you manually select **Run workflow** after uploading the files to a repository. It uploads a **private CI artifact attached to the workflow run**, not a GitHub Release. Nothing in this repository automatically publishes an application.

1. Upload the project files to GitHub, including the hidden `.github` directory. Do not upload `.env`, token files, or your real accounts data.
2. Go to **Actions → Pebble Linux AppImage (manual)**.
3. Choose **Run workflow**, only when you want a test build.
4. The workflow checks source completeness, frontend types, Rust compilation/tests, and then packages an AppImage.
5. The resulting `.AppImage` appears in **Artifacts** if and only if every earlier step succeeds. Source code may still need fixes if the Rust compiler reports errors.

The workflow uses Ubuntu 22.04 and Node.js 22 and does not build Windows or `.deb`. It does not sign the package or create a release.

To build on your own Linux computer instead:

```bash
npm install
python3 scripts/check_source.py
npm run check
npm run build -- --bundles appimage
```

Requires Rust + Linux Tauri 2 prerequisites. The artifact is **not** a claim of full feature/runtime support.
