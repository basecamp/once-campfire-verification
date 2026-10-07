# once-campfire-verification

Shared verification and benchmarks for [Campfire](https://github.com/basecamp/once-campfire) and its Django, Laravel, Express, Elixir, Go, Rust and C implementations.

Every measured HTTP response must match its route contract: status, headers, complete decoded body, expected messages and content. Every acknowledged message write must match its exact persisted ID, body, room and search-index entry. Any invalid response or failed write audit fails the run. Browser flows check installation, live messages, editing, search, permissions, settings, invitations and session transfer.

## Run

Requires Ruby with Minitest, Rust 1.98.1, Node 22.18+, SQLite CLI, FFmpeg, Git and Docker. Keep this checkout alongside the implementation checkouts.

```sh
npm ci
npx playwright install chromium
bin/check
bin/seed
cargo build --release --locked --manifest-path loadgen/Cargo.toml
bin/benchmark --apps rails,elixir,go,rust
```

Build the implementations' production images first. Override their image names with `RAILS_IMAGE`, `DJANGO_IMAGE`, `LARAVEL_IMAGE`, `EXPRESS_IMAGE`, `ELIXIR_IMAGE`, `GO_IMAGE`, `RUST_IMAGE` and `C_IMAGE`. `--help` lists the benchmark options, including CPU affinity, seed path, route selection and output directory. The default is three alternating rounds with 16 concurrent clients. A process lock prevents overlapping benchmark runs. The fixture builder pins public Rails revision `90b3300` and generates real attachments and variants; it generates disposable signing, push and login credentials locally and refuses to overwrite an existing seed.

Against a **fresh, disposable** running app:

```sh
bin/browser --base http://127.0.0.1:3000
```

This flow creates and modifies accounts, rooms and messages. Run each implementation's own complete test suite as well: shared checks complement framework-specific tests and screenshot inventories.

Results, seeds and browser artifacts stay in ignored directories. No benchmark result files are committed. An HTTP throughput result does not measure concurrent users or WebSocket capacity.

## Provenance

Extracted from the Rust and Elixir verification work, with browser flows adapted from the Go implementation. The fixtures are built by the public Rails app. Original 37signals copyright and MIT license are retained.
