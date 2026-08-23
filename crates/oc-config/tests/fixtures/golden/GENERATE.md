# Generate fixtures golden

Fixture di folder ini **dihasilkan dari paket TS asli** yang menjadi acuan
perilaku port Rust:

- `jsonc/*.json` — `jsonc-parser@3.3.1` (`parse` fault-tolerant: value,
  kode error, offset) → dibandingkan dengan `oc_config::parse::parse_fault_tolerant`.
- `merge/*.json` — `remeda@2.26.0` `mergeDeep` (semantik yang dipakai
  `mergeConfig` di config.ts) → dibandingkan dengan `oc_config::config::merge_deep`.
- `xdg.json` — `xdg-basedir@5.1.0` (fallback home, env kosong = fallback,
  env custom honored) → dibandingkan dengan path statik `oc_global`.

## Cara regenerate

```sh
cd rust-opencode/tools/golden-gen
bun install          # jsonc-parser@3.3.1, remeda@2.26.0, xdg-basedir@5.1.0
bun run gen_jsonc.ts
bun run gen_merge.ts
bun run gen_xdg.ts
```

Script generator menulis langsung ke folder ini. Versi paket dipatok di
`tools/golden-gen/package.json`; kalau repo opencode menaikkan versi
dependency-nya (cek `packages/opencode/package.json` dan root `package.json`
catalog), naikkan juga versi di sini lalu regenerasi.

Catatan: golden untuk loader TS penuh (`Config.loadInstanceState`) ditunda —
membutuhkan instalasi dependency monorepo utuh (`bun install --frozen-lockfile`
di clone `opencode/`), yang saat ini macet di lingkungan kerja ini. Lihat
PROGRESS.md.
