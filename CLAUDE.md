# CLAUDE.md — Rust Jira CLI

Dokumen ini adalah panduan untuk Claude (dan contributor) saat bekerja di repo ini.
Baca seluruh dokumen sebelum membuat perubahan apapun.

---

## Aturan Claude — hal yang TIDAK BOLEH dilakukan

> Bagian ini khusus untuk Claude. Wajib dibaca dan dipatuhi tanpa exception.

**Claude DILARANG menjalankan perintah git apapun yang bersifat menulis ke history:**

```
# DILARANG — jangan pernah jalankan ini
git commit
git push
git tag
git merge
git rebase
git cherry-pick
git reset --hard
git stash
```

Claude hanya boleh:
- Membuat dan mengedit file di filesystem
- Menjalankan `cargo` commands untuk build/test/check
- Menjalankan `git status`, `git diff`, `git log` untuk **membaca** state saja

Semua operasi commit, push, dan tag adalah tanggung jawab pemilik repo sepenuhnya.
Kalau Claude perlu "commit sesuatu", cukup siapkan file-nya dan instruksikan
pemilik repo untuk melakukan commit sendiri.

### TASK.md — checklist kerja Claude

File `TASK.md` di root repo adalah checklist kerja Claude yang **gitignored** (tidak masuk repo).

**Claude WAJIB:**
1. Baca `TASK.md` di awal setiap sesi baru
2. Update checkbox `[ ]` → `[x]` segera setelah task selesai dan smoke test hijau
3. Kalau `TASK.md` tidak ada (misal fresh clone), buat ulang berdasarkan context percakapan atau tanya pemilik repo

Tujuan: supaya setelah compaction, Claude bisa lanjut dari titik yang benar tanpa harus
mengulang pekerjaan yang sudah selesai.

---

## Project overview

Rust CLI untuk Atlassian Jira yang menggantikan / memperbaiki keterbatasan
[jira-cli (Go)](https://github.com/ankitpokhrel/jira-cli). Fokus utama:

- Support **full custom field** — dynamic introspection via API, tidak ada YAML
  config manual
- **Attachment upload** native dari terminal (multipart/form-data)
- Kompatibel dengan **Jira REST API v3** terbaru — `/search/jql`, bulk ops,
  field schemes, priority schemes
- TUI interaktif via **ratatui** + **crossterm**
- Binary tunggal, tidak ada runtime dependency

---

## Workspace structure

```
jira-commands/
├── CLAUDE.md
├── SECURITY.md             # responsible disclosure policy
├── CHANGELOG.md            # generated/updated by release-please
├── Cargo.toml              # workspace root
├── crates/
│   ├── jira-core/          # PUBLIC LIBRARY: API client, auth, model, ADF parser
│   │   ├── Cargo.toml      # dipublish ke crates.io sebagai "jira-core"
│   │   └── src/
│   └── jira/               # BINARY: clap commands, TUI, wiring semua crate
│       ├── Cargo.toml      # dipublish ke crates.io sebagai "jira-commands"
│       └── src/
├── plugin/
│   └── .claude-plugin/     # Claude Code plugin (9 skills)
├── .github/
│   └── workflows/
│       ├── ci.yml              # fmt + clippy + test, semua push/PR ke main
│       ├── security.yml        # cargo audit, SHA-pinned
│       ├── release-please.yml  # otomatis bump versi + CHANGELOG + tag
│       └── release.yml         # build binaries + publish crates.io (trigger: tag)
├── release-please-config.json
├── .release-please-manifest.json
└── tests/                  # integration tests (cross-crate)
```

### Dua cara install untuk end user

```bash
# Cara 1 — via cargo (dari crates.io)
cargo install jira-commands

# Cara 2 — download binary langsung dari GitHub Releases
# (tidak perlu Rust toolchain)
```

### Pemisahan tanggung jawab crate

**`jira-core`** — public API, harus stabil dan terdokumentasi:
- `JiraClient` — semua HTTP call ke Jira API
- Model types (`Issue`, `Field`, `Sprint`, dll.)
- ADF parser dan renderer
- Auth (token, PAT, keyring)
- Error types (`thiserror`)

Orang lain bisa pakai `jira-core` sebagai library dependency tanpa harus pakai CLI-nya:
```toml
[dependencies]
jira-core = "0.4"
```

**`jira/` (binary)** — tidak perlu stabil sebagai API publik:
- Semua `clap` command definitions
- TUI dengan `ratatui` + `crossterm`
- Interactive prompt dengan `inquire`
- Hanya re-export hal yang perlu dari `jira-core`

---

## Tech stack

| Kebutuhan | Crate | Keterangan |
|---|---|---|
| CLI arg parsing | `clap` (derive) | Subcommands, help generation |
| TUI framework | `ratatui` + `crossterm` | Fork aktif dari tui-rs |
| Interactive prompt | `inquire` | Select, input, confirm, Text |
| Async HTTP | `reqwest` (async, multipart, rustls-tls) | Native multipart untuk attachment |
| Async runtime | `tokio` | Full features |
| Serialisasi | `serde` + `serde_json` | JSON API response |
| Config | `figment` + `toml` | Multi-source config (file + env) |
| Token storage | Config file `~/.config/jira/config.toml` chmod 600 | Tidak pakai keyring (cross-platform) |
| Path resolution | `dirs` | XDG-compliant config path |
| Markdown → ADF | `comrak` | CommonMark + GFM, konversi ke ADF |
| MIME detection | `mime_guess` | Untuk attachment upload |
| Browser open | `open` | `jira issue view --open`, TUI `o` key |
| Progress | `indicatif` | Spinner dan progress bar (suppress di non-TTY) |
| Error handling | `anyhow` (app) + `thiserror` (lib) | |
| Logging | `tracing` + `tracing-subscriber` | Debug mode via `--verbose` |

---

## Jira API — aturan implementasi

### Endpoint yang WAJIB dipakai (API v3 terbaru)

```
# Search — JANGAN pakai /rest/api/3/search (sudah mati Okt 2025)
GET/POST /rest/api/3/search/jql        ← ini yang benar
POST     /rest/api/3/search/approximate-count   ← untuk estimasi total

# Field system — JANGAN pakai /fieldconfiguration* (dihapus Juli 2026)
GET  /rest/api/3/projects/fields       ← field tersedia per project
PUT  /rest/api/3/field/association     ← associate field ke project
DEL  /rest/api/3/field/association     ← remove association

# Priority — JANGAN assume priority global
GET  /rest/api/3/priorityscheme        ← priority per project scheme
```

### Endpoint yang JANGAN diimplementasi

```
GET/POST /rest/api/3/search            ← sudah dimatikan
GET/POST /rest/api/3/fieldconfiguration*  ← dihapus Juli 2026
```

### Pagination — cursor-based (bukan offset)

```rust
// BENAR
let mut next_page_token: Option<String> = None;
let mut iterations = 0;
let max_iterations = 500; // safeguard infinite loop bug Atlassian

loop {
    iterations += 1;
    if iterations > max_iterations { break; }

    let resp = client.search_jql(&jql, next_page_token.as_deref()).await?;
    // proses resp.issues ...

    match resp.next_page_token {
        Some(token) => next_page_token = Some(token),
        None => break,
    }
}

// SALAH — startAt tidak ada lagi
client.search(jql, start_at).await?; // ❌
```

### Rate limiting — wajib handle

Setiap HTTP client call harus handle 429 dengan `Retry-After`:

```rust
// Di jira-core/src/client.rs
if response.status() == 429 {
    let retry_after = response
        .headers()
        .get("Retry-After")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    tokio::time::sleep(Duration::from_secs(retry_after)).await;
    // retry request
}
```

### Field resolution — selalu runtime, tidak hardcode

```rust
// BENAR — fetch field schema saat runtime, cache dengan TTL
let fields = client.get_project_fields(&project_key).await?;

// SALAH — jangan hardcode assumption tentang field apapun
let story_points = issue.fields["customfield_10016"]; // ❌
```

### Async task pattern — untuk operasi berat

Beberapa operasi (archive, bulk ops besar) bersifat async di sisi Jira:

```rust
// Submit → poll → selesai
let task_url = client.archive_issues(&jql).await?;
loop {
    let status = client.poll_task(&task_url).await?;
    match status {
        TaskStatus::Complete => break,
        TaskStatus::InProgress(pct) => spinner.set_message(format!("{}%", pct)),
        TaskStatus::Failed(msg) => return Err(anyhow!(msg)),
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

### Deteksi Jira tier sebelum pakai fitur premium

```rust
// Plans API hanya Jira Premium — detect dulu
let info = client.get_server_info().await?;
if !info.is_premium() {
    return Err(UserError::FeatureRequiresPremium("Plans"));
}
```

### Dua base URL — jangan campur

```rust
// Platform API
const PLATFORM_BASE: &str = "/rest/api/3";

// Jira Software / Agile API (sprint, board)
const AGILE_BASE: &str = "/rest/agile/1.0";

// JANGAN hardcode langsung di setiap call
// Pakai method di client yang sudah tahu endpoint mana yang benar
```

---

## Commit message convention (untuk pemilik repo)

> Claude tidak melakukan commit. Bagian ini adalah panduan untuk pemilik repo.

Pakai **Conventional Commits**:

```
feat: add attachment upload command
fix: handle nextPageToken infinite loop
docs: update CLAUDE.md with rate limit rules
chore: bump reqwest to 0.12
refactor: extract ADF renderer to separate module
test: add integration test for bulk transition
ci: add Windows target to release workflow
perf: cache field schema per project with TTL
```

Format: `<type>(<scope optional>): <description singkat>`

Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`, `perf`

Commit message dalam bahasa Inggris.

---

## Checklist sebelum commit/push (tanggung jawab pemilik repo)

Claude akan menjalankan smoke test dan melaporkan hasilnya.
Pemilik repo yang memutuskan apakah akan commit dan push.

### 1. Smoke test — Claude jalankan ini, laporkan hasilnya

```bash
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all
```

Kalau ada yang merah, Claude fix dulu sampai semua hijau sebelum lapor ke pemilik repo.

### 2. Conflict check — pemilik repo jalankan ini sebelum push

```bash
git fetch origin
git status

# Kalau ada divergence
git rebase origin/main
# resolve conflict kalau ada
git rebase --continue
```

Gunakan `rebase`, bukan `merge`, supaya history tetap linear.

### 3. Version bump

**Jangan pernah manual bump versi.** Release-please menentukan versi berikutnya
secara otomatis berdasarkan tipe commit:

| Commit type | Bump yang dihasilkan |
|---|---|
| `feat:` | MINOR (`0.4.1 → 0.5.0`) |
| `fix:`, `perf:`, `refactor:` | PATCH (`0.4.1 → 0.4.2`) |
| `feat!:` atau `BREAKING CHANGE:` di footer | MAJOR (`0.4.1 → 1.0.0`) |
| `chore:`, `docs:`, `ci:`, `test:` | Tidak trigger release baru |

Release-please akan bump: `crates/jira-core/Cargo.toml`, `crates/jira/Cargo.toml`,
dan `plugin/.claude-plugin/plugin.json` sekaligus dalam satu Release PR.

---

## GitHub Actions

Semua workflow menggunakan **SHA-pinned actions** untuk supply-chain security.
Lihat file aktual di `.github/workflows/` — jangan salin YAML dari dokumen ini.

### `ci.yml` — quality gate di setiap push/PR

Trigger: semua push ke `main` dan semua PR ke `main`.
Matrix: ubuntu, macos, windows.
Steps: `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test --all` → `cargo build --all`

### `security.yml` — dependency audit

Trigger: setiap push ke `main`.
Menjalankan `cargo audit` terhadap RustSec Advisory Database.

### `release-please.yml` — otomatis versi + CHANGELOG + tag

Trigger: setiap push ke `main`.
Menganalisis Conventional Commits lalu membuat/mengupdate Release PR.
Saat Release PR di-merge: push tag → trigger `release.yml`.

Butuh secret `RELEASE_PLEASE_TOKEN` (fine-grained PAT: Contents + Pull requests write).
Fallback ke `GITHUB_TOKEN` tapi tag yang dibuat tidak akan trigger `release.yml`.

### `release.yml` — build + publish (trigger: tag `v*`)

1. Validasi tag cocok dengan versi di `Cargo.toml`
2. Build binary 5 platform (linux x86/arm64, macos x86/arm64, windows x86)
3. Publish `jira-core` ke crates.io → tunggu sparse index → publish `jira-commands`
4. Create GitHub Release dengan binaries + checksums

### Secrets yang perlu di-setup

| Secret | Isi | Di mana set |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | API token dari crates.io | Settings → Secrets → Actions |
| `RELEASE_PLEASE_TOKEN` | Fine-grained PAT (Contents + PRs write) | Settings → Secrets → Actions |

---

## Branch protection rules (setup manual di GitHub)

Setelah repo public, aktifkan di `Settings → Branches → Add rule` untuk branch `main`:

- [x] Require status checks to pass before merging
  - Pilih: `Check (ubuntu-latest)`, `Check (macos-latest)`, `Check (windows-latest)`
- [x] Require branches to be up to date before merging
- [x] Do not allow bypassing the above settings

---

## Development workflow

```bash
# Clone dan setup
git clone https://github.com/mulhamna/jira-commands
cd jira-commands
cargo build --all

# Develop — Claude bisa bantu edit file, jalankan test, fix issues

# Claude akan jalankan smoke test dan kasih tahu hasilnya:
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all

# Kalau semua hijau — pemilik repo yang commit dan push:
# git add .
# git commit -m "feat: ..."
# git push origin main
```

---

## Roadmap singkat

| Phase | Fokus | Status |
|---|---|---|
| 1 — Foundation | Auth, config, HTTP client, search (cursor pagination), issue CRUD, TUI dasar | **Done** |
| 2 — Custom field & Attachment | Dynamic field introspection, semua field type, upload file/image | **Done** |
| 3 — Bulk ops & Advanced TUI | Bulk edit/transition, worklog CRUD, JQL builder interaktif | **Done** |
| 4 — Power features | Plans API, archive, raw API passthrough, Claude Code plugin | **Done** |
| 5 — UX & Automation | Improved `--help`, non-interactive create/update, `--json` mode, `bulk-create`, `clone`, `batch`, TUI edit actions (c/e/a/w/l/m/u) | **Done** |

---

## Alur release — release-please (otomatis)

> **Jangan pernah manual bump version, manual update CHANGELOG, atau manual push tag.**
> Semua diurus oleh **release-please** via CI/CD.

### Cara kerjanya

1. Push commit ke `main` dengan **Conventional Commits** (`feat:`, `fix:`, `chore:`, dll.)
2. release-please otomatis buat/update sebuah "Release PR" yang:
   - Bump versi di `crates/jira-core/Cargo.toml`, `crates/jira/Cargo.toml`, `plugin/.claude-plugin/plugin.json`
   - Generate entry baru di `CHANGELOG.md`
3. Pemilik repo **merge** Release PR
4. release-please push tag → `release.yml` trigger:
   - Build binary 5 platform
   - Publish `jira-core` ke crates.io
   - Publish `jira-commands` ke crates.io
   - Create GitHub Release dengan binaries + checksums

### Yang perlu dilakukan sebelum push ke main

```bash
# 1. Smoke test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all

# 2. Commit dengan Conventional Commits — ini yang release-please baca untuk bump versi
git add .
git commit -m "feat: add TUI edit actions and bulk-create command"
git push origin main

# 3. Tunggu CI hijau → release-please buat/update Release PR otomatis
# 4. Review dan merge Release PR → tag + release otomatis
```

### Aturan crates.io

- **Jangan publish manual** — publish selalu via GitHub Actions setelah Release PR di-merge
- Urutan publish: `jira-core` dulu → tunggu 90 detik → baru `jira-commands`
- Kalau publish gagal di tengah jalan: cek apakah salah satu sudah terpublish di crates.io — versi yang sama tidak bisa di-publish ulang
- CI hanya menjalankan `cargo publish --dry-run -p jira-core`. `jira-commands` tidak di-dry-run karena depend ke `jira-core` versi baru yang belum ada di crates.io

### Aturan Claude Code plugin marketplace

- Versi di `plugin/.claude-plugin/plugin.json` di-bump otomatis oleh release-please (via `extra-files` di `release-please-config.json`)
- Plugin menggunakan binary `jira` yang sudah terinstall — pastikan README dokumentasikan `cargo install jira-commands` sebagai prerequisite
- Setiap ada skill baru atau perubahan perilaku skill, update description di `plugin/skills/<skill>/SKILL.md` dan update tabel di README.md

---

## Setiap kali update CLAUDE.md ini

Kalau ada perubahan arsitektur, aturan baru, atau temuan soal Jira API:

1. Update bagian yang relevan di dokumen ini
2. Tambahkan entry di bawah dengan format:

```
## Changelog CLAUDE.md

| Tanggal | Perubahan |
|---|---|
| 2025-04-14 | Initial version |
| YYYY-MM-DD | <deskripsi perubahan> |
```

---

## Changelog CLAUDE.md

| Tanggal | Perubahan |
|---|---|
| 2026-04-14 | Initial version — hasil analisis jira-commands gaps + Jira API v3 terbaru |
| 2026-04-14 | Hapus jira-tui sebagai crate terpisah, TUI masuk ke binary crate |
| 2026-04-14 | Tambah aturan Claude dilarang git commit/push/tag — hanya pemilik repo |
| 2026-04-15 | Phase 1 selesai — auth, config, HTTP client, issue CRUD, TUI dasar |
| 2026-04-15 | Hapus keyring, token disimpan di config file chmod 600 (cross-platform) |
| 2026-04-15 | Tambah `auth update` untuk ganti URL/email/token tanpa login ulang |
| 2026-04-15 | Fix JQL default: fallback ke `assignee = currentUser()` jika tanpa project |
| 2026-04-15 | Fix Cargo.toml untuk publish crates.io: version di path dep + metadata fields |
| 2026-04-15 | Publish ke crates.io trigger via tag `v*`, BUKAN push ke main |
| 2026-04-15 | Rename crate binary dari "jira" ke "jira-commands" (nama "jira" sudah dipakai di crates.io) |
| 2026-04-15 | Phase 2 selesai — FieldKind/FieldValue, FieldCache, attachment upload, `issue attach`, `issue fields`, create dengan dynamic field prompts |
| 2026-04-15 | Phase 3 & 4 selesai — worklog CRUD, bulk transition/update, archive, JQL builder, `jira api` raw passthrough, `jira plan list`; versi bump ke 0.2.0 |
| 2026-04-15 | Tambah Claude Code plugin di `plugin/` — 9 skills (list, view, create, transition, worklog, bulk-transition, attach, jql, api); versi bump ke 0.3.0 |
| 2026-04-16 | Fix 204 No Content handling; fix assignee ke accountId (resolve email→accountId via /user/search, support "me" via /myself); raw_request return Option<Value>; quiet spinner/progress bar saat non-TTY; tambah CHANGELOG.md; versi bump ke 0.4.0 |
| 2026-04-16 | Fix TUI JQL search: tambah f.set_cursor_position() di render_search_bar() supaya cursor terminal muncul saat user ketik JQL; hapus fake █ cursor di footer; update plugin list-issues skill |
| 2026-04-17 | Phase B–E selesai — improved --help, non-interactive create/update, bulk-create, clone, batch, --json mode, TUI edit actions (c/e/a/w/l/m/u), CI security job, SECURITY.md |
| 2026-04-17 | Ganti alur release: hapus manual version bump/tag, gunakan release-please via CI/CD — update CLAUDE.md, TASK.md |
