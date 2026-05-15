# Firefly Indexer

Adobe Firefly training-data indexer — turn a folder of photos into a structured Excel index ready for Adobe Firefly model training.

Made by **SUNNY (Yu Hee-jung)** · © 2026.

---

## What it does

- Scans a folder of photos (`.jpg` / `.png`)
- Groups them into scenes (1 `full_scene` + N isolated reference shots)
- Calls a local Claude vision model to produce, for each photo:
  - English **caption** (15–30 words)
  - English **edit_instruction** (placement, removal, isolation)
  - `characterId`, `edit_type`, `title` — matching the Adobe Firefly training schema
- Writes a 10-column `scenes` sheet + 4-column `characterid` sheet into an `.xlsx`
- Optionally:
  - **Recheck** — auto-validate form rules · AI-slop detection · per-row vision round-trip score (caption vs image, edit_instruction vs image)
  - **Rename** — bulk-rename non-conforming filenames into the standard pattern, copies to a sidecar folder (originals never modified)

## How it runs

- Single desktop app (Tauri 2.0). Windows `.msi`/`.exe` + macOS `.dmg`/`.app`.
- Uses the **user's own Claude Pro/Max subscription** through the Claude Code CLI installed on the user's PC. No external API key required.

## Download

See the **Downloads** page (link will be added once the first release is published).

## Build (developers)

```bash
npm install
npm run tauri build
```

Cross-platform builds run automatically via GitHub Actions on tag push (`v*`).

---

© 2026 SUNNY. All rights reserved.
