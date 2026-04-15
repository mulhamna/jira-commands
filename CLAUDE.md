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
jira-cli/
├── CLAUDE.md
├── Cargo.toml              # workspace root
├── crates/
│   ├── jira-core/          # PUBLIC LIBRARY: API client, auth, model, ADF parser
│   │   ├── Cargo.toml      # dipublish ke crates.io sebagai "jira-core"
│   │   └── src/
│   └── jira/               # BINARY: clap commands, TUI, wiring semua crate
│       ├── Cargo.toml      # dipublish ke crates.io sebagai "jira-cli"
│       └── src/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
└── tests/                  # integration tests (cross-crate)
```

### Dua cara install untuk end user

```bash
# Cara 1 — via cargo (dari crates.io)
cargo install jira-cli

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
jira-core = "0.1"
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
| Interactive prompt | `inquire` | Select, input, confirm |
| Async HTTP | `reqwest` (async, multipart) | Native multipart untuk attachment |
| Async runtime | `tokio` | Full features |
| Serialisasi | `serde` + `serde_json` | JSON API response |
| Config | `figment` + `toml` | Multi-source config (file + env) |
| Token storage | `keyring` | OS keychain (macOS/Linux/Windows) |
| Path resolution | `dirs` | XDG-compliant config path |
| Markdown parser | `comrak` | CommonMark + GFM, konversi ke ADF |
| Syntax highlight | `syntect` | Code block di issue view |
| Progress | `indicatif` | Spinner dan progress bar |
| Error handling | `anyhow` (app) + `thiserror` (lib) | |
| Logging | `tracing` + `tracing-subscriber` | Debug mode |

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

### 3. Version bump — kapan dan bagaimana

Versi mengikuti **Semantic Versioning (semver)**:

| Jenis perubahan | Bump |
|---|---|
| Breaking change di public API `jira-core` | MAJOR (`1.0.0 → 2.0.0`) |
| Fitur baru backward-compatible | MINOR (`0.1.0 → 0.2.0`) |
| Bug fix, patch, performance | PATCH (`0.1.0 → 0.1.1`) |
| Dependency update non-breaking | PATCH |

Claude boleh edit `Cargo.toml` untuk bump versi, tapi **commit tetap dilakukan
pemilik repo**.

```bash
# Setelah Claude edit Cargo.toml, pemilik repo jalankan:
cargo update -p jira-core   # update Cargo.lock
# lalu commit manual
```

Kalau `jira-core` versinya naik, `jira/Cargo.toml` yang depend ke `jira-core`
juga harus ikut di-update.

---

## GitHub Actions

### CI (`ci.yml`) — jaga branch main

Trigger: setiap push ke `main` dan setiap PR ke `main`.

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Tests
        run: cargo test --all

      - name: Build
        run: cargo build --all
```

### Release (`release.yml`) — publish ke crates.io + GitHub Releases

Trigger: push tag `v*` (contoh: `v0.1.0`).

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

permissions:
  contents: write

jobs:
  build-binaries:
    name: Build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: jira-linux-x86_64
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: jira-linux-aarch64
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: jira-macos-x86_64
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: jira-macos-aarch64
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            artifact: jira-windows-x86_64.exe

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross (Linux aarch64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: Build (cross)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: cross build --release --target ${{ matrix.target }} -p jira

      - name: Build (native)
        if: matrix.target != 'aarch64-unknown-linux-gnu'
        run: cargo build --release --target ${{ matrix.target }} -p jira

      - name: Rename binary (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cp target/${{ matrix.target }}/release/jira ${{ matrix.artifact }}

      - name: Rename binary (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cp target/${{ matrix.target }}/release/jira.exe ${{ matrix.artifact }}

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}

  publish-crates:
    name: Publish to crates.io
    runs-on: ubuntu-latest
    needs: build-binaries
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Publish jira-core
        run: cargo publish -p jira-core --token ${{ secrets.CARGO_REGISTRY_TOKEN }}
      # Tunggu crates.io index sebelum publish binary yang depend ke jira-core
      - run: sleep 30
      - name: Publish jira-cli (binary crate)
        run: cargo publish -p jira --token ${{ secrets.CARGO_REGISTRY_TOKEN }}

  create-release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    needs: [build-binaries, publish-crates]
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts/

      - name: Generate checksums
        run: |
          cd artifacts
          for dir in */; do
            file="${dir%/}"
            sha256sum "$file/$file" >> ../checksums.txt 2>/dev/null || \
            sha256sum "$file/$file.exe" >> ../checksums.txt 2>/dev/null || true
          done

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            artifacts/**/*
            checksums.txt
```

### Cara trigger release (pemilik repo)

```bash
# Pastikan main sudah clean
git status

# Bump versi di Cargo.toml (bisa minta Claude untuk edit file-nya)
# lalu commit manual

# Tag dengan versi baru
git tag v0.1.0

# Push tag — ini yang trigger release workflow
git push origin v0.1.0
```

### Secrets yang perlu di-setup di GitHub repo

| Secret name | Isi | Di mana set |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | Token dari crates.io | Settings → Secrets → Actions |

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
git clone https://github.com/<username>/jira-cli
cd jira-cli
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
| 1 — Foundation | Auth, config, HTTP client, search (cursor pagination), issue CRUD, TUI dasar | Planned |
| 2 — Custom field & Attachment | Dynamic field introspection, semua field type, upload file/image | Planned |
| 3 — Bulk ops & Advanced TUI | Bulk edit/transition, worklog CRUD, JQL builder interaktif | Planned |
| 4 — Power features | Plans API, archive, raw API passthrough, plugin scripting | Planned |

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
| 2026-04-14 | Initial version — hasil analisis jira-cli gaps + Jira API v3 terbaru |
| 2026-04-14 | Hapus jira-tui sebagai crate terpisah, TUI masuk ke binary crate |
| 2026-04-14 | Tambah aturan Claude dilarang git commit/push/tag — hanya pemilik repo |
