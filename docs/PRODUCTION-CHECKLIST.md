# PBR Studio – Production-Ready Checklist

Step-by-step implementation checklist for Cursor's vibe coding workflow. All tasks are **offline, local-only** — no backend, cloud storage, or database.

---

## CORE ENGINE (pbr-core)

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 1 | **PDF font lookup for Windows/macOS** – Add platform-specific font paths to `find_font_path()` (e.g. `C:\Windows\Fonts`, `~/Library/Fonts`) or bundle LiberationSans in the binary | High | None | PDF export works on all platforms |
| 2 | **Bundle fallback font** – Embed a minimal TTF (e.g. LiberationSans-Regular) as `include_bytes!` when system fonts not found | Medium | Task 1 | Reliable PDF on any OS |
| 3 | **EXR input support** – Add `exr` crate, implement `ImageLoader` path for `.exr` files, convert to RGBA8 for analysis | Low | None | Load EXR textures |
| 4 | **AI module ONNX integration** – Add optional `tract` or `ort` crate behind `ai` feature; load local ONNX model for classification/anomaly | Low | None | ML-based classification when model present |
| 5 | **Material report ai_insights** – Ensure `run_advanced_analysis` output integrates with `MaterialReport` JSON for UI consumption | Medium | None | JSON with duplicates, cross-material, tileability |

---

## CLI (pbr-cli)

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 6 | **`export-report --format json`** – Add JSON format to `cmd_export_report`; write batch JSON array of MaterialReports to output file | High | None | Batch JSON report file |
| 7 | **`aggregate-report` command** – New command: scan root, collect all materials, output single aggregated JSON (summary stats, per-material scores, trends) | Medium | None | Aggregated JSON report |
| 8 | **Pre-commit git check** – Document that `pre-commit` requires git repo; add clear error message when not in git | Medium | None | Clear UX when run outside git |
| 9 | **`optimize --config`** – Allow loading preset from JSON/TOML config file (target resolution, LOD levels) | Low | Plugin system | Config-driven export |
| 10 | **`analyze` output to file** – Add `--output <path>` to `analyze` for saving JSON to file | Low | None | Saved advanced analysis JSON |

---

## DESKTOP UI (pbr-studio-ui)

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 11 | **Multi-folder picker** – Change folder picker to `multiple: true` where supported; allow selecting multiple material folders at once | High | None | Add several folders in one action |
| 12 | **"Desktop only" message** – When `!isTauri`, show full-screen overlay: "PBR Studio runs as a desktop app. Use Tauri build or CLI." | High | None | Clear message in browser |
| 13 | **Advanced analysis panel** – Add Tauri command `run_advanced_analysis`, call from UI; new panel or tab for duplicates, cross-material trends, tileability report | High | Task 5 (core), new Tauri command | UI panel with duplicates/trends |
| 14 | **Fix tileability from UI** – Add Tauri command wrapping `fix_tileability_with_report`; button in ValidationPanel or new panel to apply fix and save | Medium | New Tauri command | Fixed texture saved to chosen path |
| 15 | **Real-time score in material list** – Score already shown; ensure it updates immediately after analysis without refresh | Low | None | Live score display |
| 16 | **HDRI preset selector** – Already present in Viewport3D; verify all presets load correctly (studio, sunset, dawn, etc.) | Low | None | Working HDRI presets |
| 17 | **Material comparison** – Already present (compare mode); verify side-by-side viewport and labels work | Low | None | A/B comparison |
| 18 | **Batch export UI** – Already supports export report for all materials; document and verify batch export preset flow | Low | None | Batch preset export |
| 19 | **Undo/redo** – Already implemented in MaterialList; verify add/remove undo works | Low | None | Undo/redo for material list |

---

## CROSS-PLATFORM BUILDS

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 20 | **Document minimum glibc** – Add to BUILD-APPIMAGE or README: "Built on Ubuntu 22.04; requires glibc 2.31+ (or similar)" | Medium | None | Clear compatibility bounds |
| 21 | **macOS code signing** – Configure Tauri for ad-hoc or Apple Developer signing; document in BUILD-MACOS | Low | Apple Developer account (optional) | DMG opens without Gatekeeper warning |

**See also**: [BUILD-APPIMAGE.md](BUILD-APPIMAGE.md) (glibc validation), [BUILD-MACOS.md](BUILD-MACOS.md) (Gatekeeper/code signing), [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) (pre-release steps).
| 22 | **Cross-platform test matrix** – Add optional CI job that runs `pbr-cli check`, `batch-check`, `report --json` on Windows/macOS runners | Medium | None | CI asserts CLI works per OS |
| 23 | **AppImage fuse3 fallback** – Document libfuse2 requirement; consider fuse3 compatibility if feasible | Low | None | Broader Linux support |

---

## OFFLINE COMPLIANCE

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 24 | **Audit all network usage** – Grep for fetch, http, XMLHttpRequest, WebSocket in src/; confirm none in app code | High | None | Verified no network calls |
| 25 | **Settings storage note** – Already present ("All settings saved locally"); ensure no cloud sync options ever added | Low | None | UX confirms offline |
| 26 | **Plugin paths** – Document that plugins load from local paths only (`~/.config/pbr-studio/plugins`, `PBR_STUDIO_PLUGINS`) | Low | None | Clear plugin docs |

---

## DOCUMENTATION

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 27 | **README CLI table** – Ensure all commands documented: check, batch-check, pre-commit, optimize, batch-optimize, report, export-report, analyze, fix-tileability, audit-log | High | None | Complete CLI reference |
| 28 | **README export-report** – Add `export-report` examples with `--format html|pdf`, `--output`, `--track` | High | None | Export-report usage |
| 29 | **README audit-log** – Add `audit-log --limit N --json` example | Medium | None | Audit log usage |
| 30 | **UI usage guide** – Create `docs/UI-GUIDE.md`: layout, drag-drop, analyze, export presets, compare mode, settings, audit log | High | None | UI-GUIDE.md |
| 31 | **CLI quickstart** – Create `docs/CLI-QUICKSTART.md`: common workflows (validate, batch check, optimize, pre-commit, report) | High | None | CLI-QUICKSTART.md |
| 32 | **Plugin tutorial** – Expand `docs/plugins/README.md` with step-by-step: create plugin.json, define rules, add preset, test | Medium | None | Plugin tutorial |
| 33 | **Advanced analysis docs** – Document `analyze` and `fix-tileability` in README; explain duplicate detection, cross-material, tileability | Medium | None | Advanced analysis docs |
| 34 | **Pre-commit caveats** – Document that pre-commit requires git; add Troubleshooting section | Medium | None | Pre-commit docs |
| 35 | **Build workflow** – All doc references point to `ci-release.yml`; verify BUILD-APPIMAGE, BUILD-MACOS, BUILD-WINDOWS | Medium | None | ✅ Done |

---

## AUDIT LOGS

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 36 | **Audit log persistence** – Verify `~/.config/pbr-studio/audit.json` (or `PBR_STUDIO_AUDIT_PATH`) used; max 1000 entries | Low | None | Audit log file |
| 37 | **Certified badge** – Verify badge written to `.pbr-studio/certified.svg` when validation passes | Low | None | Badge file in material folder |
| 38 | **Audit log in reports** – Optionally include last audit entry or certified status in MaterialReport JSON | Low | None | Report includes audit info |

---

## PLUGIN / CUSTOM RULE SYSTEM

| # | Task | Priority | Dependencies | Expected Output |
|---|------|----------|--------------|-----------------|
| 39 | **Plugin discovery** – Verify `PluginLoader` loads from `~/.config/pbr-studio/plugins`, `PBR_STUDIO_PLUGINS`, project `plugins/` | Low | None | Plugins load correctly |
| 40 | **Plugin docs** – Document RuleCondition types (RequiredMaps, MaxResolution, Script, etc.) with examples | Medium | None | Plugin reference |
| 41 | **Script plugin security** – Document that Script plugins run local commands; warn about untrusted configs | Low | None | Security note |

---

## IMPLEMENTATION ORDER (Suggested)

**Phase 1 – Critical for release**
- 1, 6, 11, 12, 27, 28, 30, 31

**Phase 2 – High value**
- 2, 5, 13, 14, 24, 32, 33, 35

**Phase 3 – Polish**
- 3, 4, 7, 8, 9, 10, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 29, 34, 36, 37, 38, 39, 40, 41

---

## QUICK REFERENCE: COMMANDS & OUTPUTS

| Command | Output |
|---------|--------|
| `pbr-cli check <folder> --ci` | JSON to stdout, exit 1 if fail |
| `pbr-cli batch-check <root> --ci` | JSON to stdout, exit 1 if any fail |
| `pbr-cli report <folder> --json` | MaterialReport JSON to stdout |
| `pbr-cli export-report <folders> --format html|pdf|json --output <path>` | HTML/PDF/JSON file (local only) |
| `pbr-cli analyze <root> --tileability` | Advanced analysis JSON to stdout |
| `pbr-cli fix-tileability <path> --output <path>` | Fixed texture file |
| `pbr-cli audit-log [--limit N] [--json] [--output FILE] [--format json|text]` | Audit entries to stdout or file (JSON/text) |

---

## NOTES FOR CURSOR WORKFLOW

- Each task can be implemented as a focused edit session.
- For Tauri commands: add in `src-tauri/src/lib.rs`, register in `invoke_handler`, add TypeScript types if needed.
- For pbr-core: add to `lib.rs` exports when adding new public APIs.
- Always verify offline: no `fetch`, `https`, or cloud APIs in app code.
- Test CLI with `cargo run -p pbr-cli -- <args>`.
- Test UI with `cd pbr-studio-ui && npm run tauri:dev`.
