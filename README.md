# rust-opencode

Port Rust dari [`sst/opencode`](https://github.com/sst/opencode) — coding agent
CLI yang aslinya ditulis dalam TypeScript + Bun + Effect — yang dikerjakan
bertahap per sprint mengikuti paket dokumen di `../rust-opencode-plan/`.

Aturan inti proyek:

1. **Behavior identik** dengan versi TS asli untuk setiap modul yang sudah
   di-port, dibuktikan lewat golden test (bukan cuma "kelihatan mirip").
2. **Nama identifier tetap dapat ditelusuri balik** ke nama TS asli; hanya
   casing convention yang berubah (lihat `01_NAMING_CONVENTION.md` di folder
   plan). Pemetaan lengkap dicatat per crate.
3. Bug/keanehan di source asli TIDAK diam-diam diperbaiki — dicatat di
   [DEVIATIONS.md](DEVIATIONS.md), default-nya direplikasi apa adanya.

## Struktur workspace

Satu crate Rust ≈ satu modul TS di `packages/opencode/src/`:

| Crate | Peran |
|---|---|
| `oc-global` | Utilitas global (`src/util/`) |
| `oc-config` | Konfigurasi (`src/config/`) |
| `oc-auth` | Autentikasi provider (`src/auth/`) |
| `oc-storage` | Persistence (`src/storage/`) |
| `oc-permission` | Sistem permission (`src/permission/`) |
| `oc-tool` | Tool bawaan: read/write/edit/shell/dll (`src/tool/`) |
| `oc-provider` | Integrasi LLM provider (`src/provider/`) |
| `oc-agent` | Definisi agent (`src/agent/`) |
| `oc-session` | Data model + prompt loop session (`src/session/`) |
| `oc-server` | HTTP server & API (`src/server/`) |
| `oc-mcp` | MCP client (`src/mcp/`) |
| `oc-lsp` | LSP client (`src/lsp/`) |
| `oc-cli` | Binary utama `rust-opencode` (`src/cli/`) |

## Dokumen penting

- [NAMING_MAP.md](NAMING_MAP.md) — index pemetaan nama TS → Rust, link ke map
  per crate.
- [PROGRESS.md](PROGRESS.md) — status sprint 0–16.
- [DEVIATIONS.md](DEVIATIONS.md) — deviasi behavior dari source asli.

## Build & check

```sh
cargo build --workspace     # build semua crate
./scripts/check.sh          # fmt --check + clippy -D warnings + test (jalankan tiap akhir sprint)
```

Menjalankan binary:

```sh
cargo run -p oc-cli
```
