# Campfire performance and verification, 2026-10-07

All eight implementations were measured with the public [shared verification harness](https://github.com/basecamp/once-campfire-verification).
Every measured response passed its content contract, and every acknowledged HTTP message
passed the exact persisted-write audit. Current results follow the completed functional checks below.
The run used harness revision [`f94e32e`](https://github.com/basecamp/once-campfire-verification/commit/f94e32e14e0de9ab1171409daeda94a505d22f9f).

22 performance pull requests are confirmed merged. The useful cache work from the separately
closed Laravel #1 is retained and credited. Merge author and committer are
`GPT on behalf of DHH <2741+dhh@users.noreply.github.com>`; original contributor commits remain intact.
38 proposals were reviewed across Rails, Rust, Go, Elixir, Laravel and Express. Django and the
public [C fork](https://github.com/basecamp/once-campfire-c) are included in the current comparison.

## Current production HTTP results

Measured with 16 concurrent clients on an AMD Ryzen AI MAX+ 395 with 32 GB RAM,
with four hardware cores allocated to each app.

| HTTP workload (requests/sec) | Rails | [Django](https://github.com/basecamp/once-campfire-django) | [Laravel](https://github.com/basecamp/once-campfire-laravel) | [Express](https://github.com/basecamp/once-campfire-express) | [Elixir](https://github.com/basecamp/once-campfire-elixir) | [Go](https://github.com/basecamp/once-campfire-go) | [Rust](https://github.com/basecamp/once-campfire-rust) | [C](https://github.com/basecamp/once-campfire-c) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 230 | 62 | 760 | 2,622 | 942 | 31,673 | 35,484 | 141,834 |
| Messages page | 402 | 70 | 924 | 3,245 | 1,267 | 30,746 | 40,674 | 151,564 |
| Sidebar | 468 | 229 | 1,383 | 34,938 | 2,515 | 18,586 | 34,479 | 159,850 |
| Search | 399 | 118 | 1,135 | 6,613 | 1,814 | 29,765 | 34,432 | 155,456 |
| Post a message | 248 | 112 | 498 | 2,088 | 1,400 | 9,073 | 8,998 | 7,460 |

These are medians of 3 alternating rounds, with two-second warmups and
8-second samples. The applications run serially on CPUs 8–11; the generator
runs on separate physical cores 12–15. Builds, functional tests and other benchmarks were stopped.
Across the timed samples, **23,061,892 responses passed with zero request errors or invalid
responses**. The write audit verified **883,601 acknowledged warmup and timed writes**.
Peak recorded generator CPU was 176.9% of its four-core 400% capacity.

Observed minimum–maximum rates across the measured rounds:

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 229–243 | 384–409 | 465–494 | 390–407 | 244–258 |
| Django | 62–62 | 69–70 | 229–231 | 118–118 | 111–113 |
| Laravel | 751–764 | 915–931 | 1,341–1,424 | 1,131–1,137 | 497–500 |
| Express | 2,561–2,707 | 3,212–3,246 | 34,744–36,074 | 5,650–7,012 | 2,083–2,135 |
| Elixir | 940–944 | 1,257–1,268 | 2,508–2,535 | 1,814–1,815 | 1,392–1,428 |
| Go | 31,235–31,885 | 30,699–31,146 | 18,276–19,088 | 29,689–29,952 | 9,032–9,092 |
| Rust | 35,143–35,734 | 40,546–41,223 | 34,144–34,650 | 34,381–34,532 | 8,989–9,079 |
| C | 141,291–143,610 | 149,450–160,331 | 159,810–160,461 | 155,251–164,668 | 7,401–7,491 |

## Response and persistence contracts

Each application gets a fresh copy of the complete Rails parity fixture, including real users,
memberships, rich text, uploads and variants. Push and webhook fixture URLs target a closed local
port. The fixture database hash is checked after the run; SQLite integrity is checked on each
application's disposable database. The repaired C seed no longer confuses user IDs with room IDs.

Warmup and timed HTTP responses must have status 200, valid MIME and wire length, valid gzip or
identity encoding, complete decoded bodies and valid UTF-8 for textual routes. Pages must contain
the independently queried fixture's expected message IDs, order, DOM identities and message text;
sidebars must contain every accessible open room. Assets and avatars must match their preflight
bodies. Avatar preflight also decodes image pixels. Health responses must indicate a healthy app.
Posts must return a complete Turbo append for the actual room, exactly one positive message ID
and the exact unique submitted body.

Every acknowledged post, including warmups, is checked by its exact ID, room, stored rich-text
body and FTS entry after timing. Duplicate acknowledgements, missing rows, mismatched bodies,
extra writes and invalid responses fail the run. Exact repeated wire bodies can reuse a completed
content check; headers are checked on every response, and changed bodies are checked in full.
The shared harness does not accept HTTP 200 error pages as successful requests.

Raw benchmark receipts, response contracts, image labels, source status and write audits stay
local; result JSON files are not committed. Historical status-only and short diagnostic timings
were not reused in this table. There is no new before/after claim: previous baselines were measured
with a different validator. HTTP rates do not measure WebSocket capacity or simultaneous people.

## Completed functional checks

| Implementation | Completed checks | Explicit limits |
|---|---|---|
| Rails | 488 tests, 1,744 assertions; 27 Chromium system tests, 197 assertions; Rubocop and Brakeman. | Two unit skips for unsupported libvips loaders; no system-test skips. |
| Django | 50 tests, Ruff and real Chromium functional flows. | Malformed rich-text and media byte parity are not established; live browser-provider Web Push requires external subscriptions. |
| Laravel | 35 tests, 641 assertions, Pint and real Chromium functional flows. | Exact rich-text parity, broader audio/video/PDF preview coverage, restore/upgrade and live browser-provider Web Push remain unverified in its contract ledger. |
| Express | 145 tests, no skips, pinned formatter and real Chromium functional flows. | Public-site OpenGraph and live browser-provider Web Push remain unverified; malformed/legacy rich text outside the independent corpus can differ. |
| Elixir | Current 1,946-test native suite and strict compilation/formatting; one uninterrupted complete 65-gate parity run at the reviewed backend revision; rebuilt production asset manifest verified. | The complete parity run predates the latest browser fixes; current regression checks supplement it. Verification used pinned OTP 29 in a disposable checkout; no production cutover is claimed. |
| Go | Full race/vet/assets/WebSocket-fork suite; real Chromium; production backup/restore/restart; real Go/Rust cookie and message/FTS interoperability; real ACME issuance and cached HTTPS restart with the CA offline. | Strict byte-for-byte HTML/network parity remains incomplete, as documented by the port. |
| Rust | Current seed-required workspace: 775 passed; clippy with warnings denied, formatting and unused-dependency checks clean. Earlier complete five-seed browser inventories: 956 cells, 954 passes, zero failures, flakes or errors; rebuilt embedded frontend verified. | Eleven existing timing/reference-export/doc cases explicitly ignored; no missing-seed skips. Earlier browser inventories predate the latest frontend fixes; fresh shared browser regressions supplement them. Two documented manifest JSON escaping allowances. |
| C | Current 1,675-test native suite; current affected sanitizer subset (174 tests); earlier complete 1,673-test ASan/UBSan/LSan and 1,673-test Fil-C runs; threaded TSan subset; pinned media vectors; repeated four-browser HTTPS flows with real PNG/video uploads. | Complete sanitizer/Fil-C receipts predate the latest view repairs; the affected sanitizer subset is checked separately. Granted real-browser Web Push delivery is not covered in the headless environment; denied-permission UI, native encryption and local delivery were checked. Existing-install upgrades and ACME remain outside this port's contract. |

The shared browser flows exercise fresh setup, confirmed Cable subscription, two-tab live
messages without duplicates, stored-XSS protection, absolute copied permalinks, edits, search,
profile/account settings, decoded QR images, room creation/renaming, bot CRUD/API posting,
custom CSS across pages, automatic session transfer, invitations, access controls and direct-room
autocomplete. A real held sidebar reload verifies that an open New Ping editor retains its
DOM identity, draft and selected recipients; Cancel and reopening are checked too.
Framework-specific browser inventories and protocol checks supplement these flows.
Fresh functional browser runs passed for all eight measured source revisions after the final fixes.
After clicking Edit, the browser driver clicks the editable text area before filling it,
waiting for the mounted editor to become actionable. This corrects browser synchronization;
the benchmark HTTP client is unchanged.

Browser validation found and fixed live-message append errors in Django and Express, relative
copied URLs in their shared cached markup, automatic session-transfer forms in Django, Express
and Laravel, and missing account CSS plus direct-room accessible naming in Laravel. Regression
tests and fresh real-browser runs passed after these fixes.

Rails copied message permalinks now retain an explicitly configured port.
Elixir's direct-room autocomplete explicitly requests JSON so the server and browser agree on the response format.
C's custom-styles action now uses its existing complete view and layout, with a persisted round-trip regression.
C's auto-submit session-transfer form now includes its closing tag and a native regression.
Sidebar refresh preserves an open direct-room picker instead of replacing the active editor on a connection callback.
Laravel nests the direct-room list and New Ping picker in their own Turbo frame, keeping the surrounding sidebar attached during editing.


The inherited room-list controller could also abort its initial Turbo frame response by
reloading as soon as Cable connected. All eight implementations now wait for the frame load,
ignore duplicate or stale connection notifications, and still refresh after a real reconnect.
A dependency-free frontend regression checks pending loads, duplicate callbacks, disconnects,
reconnects, detached/reinserted controllers and genuine load failures. Production assets were
rebuilt and checked, including the Rust/Go embedded bodies and Elixir's compiled manifest.

## Architecture shared across implementations

All eight use bounded full-text probes with a membership-scoped fallback, select the newest
100 accessible matches by message ID and recheck permissions when hydrating results. Incremental
message refresh uses a `(room_id, updated_at)` index, and direct-room lookup uses an exact SQL
membership check. Permission tests cover sparse membership and inaccessible global matches.
Changes from frozen references are documented in each port's known differences.

All eight render complete auto-submit session-transfer forms. Rails, Rust, Go and Elixir
adopted the same closing-tag repair identified in C, while Django, Express and Laravel
already closed their forms. Native GET regressions verify the actual response, including
the transfer form's closure and method. Comparisons with frozen references permit only
that added closing tag; historical fixtures remain unchanged.

C now defaults to a 64 MiB response-body cache and at most four affinity-selected event loops.
Cache hits recheck authorization and observe external SQLite commits. This cache contributes to
the measured warm read rates; writes still perform real durable database work.

The table compares complete production configurations, rather than isolating language speed.
Rails uses Puma and Redis/Resque; Elixir retains Redis/Resque compatibility; Django uses its
one-worker ASGI default; Express uses affinity-sized HTTP workers; Laravel uses persistent
Octane workers; Go, Rust and C run their native servers. Broader opportunities in rendering,
compression, caching, job persistence and concurrency remain; this does not claim that every
architecture choice is interchangeable or has been transferred.

## Large-history Rust evidence

Two million generated messages on top of the real seed; SQLite 3.53.4; six alternating SQL samples. Broad search: coffee 43.2 → 0.28 ms, common message 677.7 → 0.26 ms. The original PR's global reverse scan regressed no-access common-word searches to about 384 ms. A 1,000-row fast-probe limit and membership-scoped fallback bound the corrected case to about 0.7 ms; sparse/no-access checks return no unauthorized results. This bounded overhead is higher than the old ~0.02 ms empty-membership query, so this is not a universal improvement.

Incremental refresh with 0/23/1,000 updated rows: ~151–155 ms → 0.07/0.07/0.36 ms. A refresh requesting the entire two-million-message history regresses ~419 → 571 ms. The optimization targets incremental refresh, not complete-history scans. These are illustrative SQL microbenchmarks, separate from the small-seed HTTP comparison.

## Reviewed proposals

Positive speed claims require independent checks. Leave-open findings below come from source
review unless a runtime regression test is explicitly stated; their advertised speeds are not
represented as independently reproduced. Competing-build measurements informed the earlier
selection but are not mixed into the current, stricter comparison.

| Pull request | Author | Review outcome |
|---|---|---|
| [once-campfire #334](https://github.com/basecamp/once-campfire/pull/334) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #331](https://github.com/basecamp/once-campfire/pull/331) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #330](https://github.com/basecamp/once-campfire/pull/330) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #329](https://github.com/basecamp/once-campfire/pull/329) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #328](https://github.com/basecamp/once-campfire/pull/328) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #326](https://github.com/basecamp/once-campfire/pull/326) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #325](https://github.com/basecamp/once-campfire/pull/325) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #324](https://github.com/basecamp/once-campfire/pull/324) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #323](https://github.com/basecamp/once-campfire/pull/323) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #322](https://github.com/basecamp/once-campfire/pull/322) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #321](https://github.com/basecamp/once-campfire/pull/321) | thomasklemm | Leave open: counter-cache totals can become stale after another implementation writes messages to the shared schema. |
| [once-campfire #319](https://github.com/basecamp/once-campfire/pull/319) | thomasklemm | Leave open: disables auto-checkpointing globally without a writer-side fallback in console/rake writers; also includes unrelated dependency/test/CI removals. |
| [once-campfire #318](https://github.com/basecamp/once-campfire/pull/318) | thomasklemm | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #316](https://github.com/basecamp/once-campfire/pull/316) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #312](https://github.com/basecamp/once-campfire/pull/312) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #311](https://github.com/basecamp/once-campfire/pull/311) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #310](https://github.com/basecamp/once-campfire/pull/310) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #296](https://github.com/basecamp/once-campfire/pull/296) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #164](https://github.com/basecamp/once-campfire/pull/164) | ashwin47 | Leave open: members_hash can become stale after membership changes; #310 provides the direct lookup without a redundant membership hash. |
| [once-campfire-rust #44](https://github.com/basecamp/once-campfire-rust/pull/44) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-rust #45](https://github.com/basecamp/once-campfire-rust/pull/45) | namespaceMarcello | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-go #2](https://github.com/basecamp/once-campfire-go/pull/2) | chemshit | Leave open: ENV GCGO is not Go's GOGC setting and has no GC tuning effect. |
| [once-campfire-go #4](https://github.com/basecamp/once-campfire-go/pull/4) | sernle | Competing implementation independently built/preflighted/timed. Full-sidebar correction extracted and credited; #9 was selected after the earlier competing-build review. |
| [once-campfire-go #5](https://github.com/basecamp/once-campfire-go/pull/5) | kidandcat | Leave open: generation caches advance only for writes through this DB instance and can reuse stale data after an external SQLite writer commits. |
| [once-campfire-go #6](https://github.com/basecamp/once-campfire-go/pull/6) | nick-potts | Competing implementation independently built/preflighted/timed. Guarded renderer incorporated and credited in #9, selected after the earlier competing-build review. |
| [once-campfire-go #7](https://github.com/basecamp/once-campfire-go/pull/7) | riscdanger | Competing implementation independently built/preflighted/timed. Alternative compression approach to #9; #9 was selected after the earlier competing-build review. |
| [once-campfire-go #8](https://github.com/basecamp/once-campfire-go/pull/8) | borovikovd | Leave open: publication relies on subscribe-time authorization/own-write disconnects; external membership revocation does not receive the same fresh check as #9. |
| [once-campfire-go #9](https://github.com/basecamp/once-campfire-go/pull/9) | nijaru | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-elixir #1](https://github.com/basecamp/once-campfire-elixir/pull/1) | zachdaniel | Leave open: competing rewrite removes durable Resque jobs/live Rails interop and moves native parsing into the BEAM. #4 retains those contracts and isolates parsing. Claimed throughput not independently reproduced. |
| [once-campfire-elixir #2](https://github.com/basecamp/once-campfire-elixir/pull/2) | kurtome | Leave open: broad response caching/optional Redis rewrite has not completed its own browser, mutation, frozen-reference and rollback gates. Selected narrower pooled implementation #4; claimed 8–10x not independently reproduced. |
| [once-campfire-elixir #3](https://github.com/basecamp/once-campfire-elixir/pull/3) | aloukissas | Leave open: alternative reader/writer pool superseded by verified #4 integration. No measured throughput claim in the PR. |
| [once-campfire-elixir #4](https://github.com/basecamp/once-campfire-elixir/pull/4) | oliver-kriska | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-elixir #5](https://github.com/basecamp/once-campfire-elixir/pull/5) | lau | Leave open: written(tables) replaces the saved WAL header after bumping only local table generations. An external change followed by an unrelated local write can be absorbed without invalidating cached external data. |
| [once-campfire-elixir #6](https://github.com/basecamp/once-campfire-elixir/pull/6) | pasilastbot | Partial adoption: literal-regex cache extracted, tested and credited. Native SQLite reads run on ordinary BEAM schedulers; previous-run slow-query detection does not bound the first slow call. Rest depends on unsafe #5 cache. |
| [once-campfire-laravel #1](https://github.com/basecamp/once-campfire-laravel/pull/1) | sneycampos | Code adopted and credited; author closed this PR without merging. |
| [once-campfire-laravel #2](https://github.com/basecamp/once-campfire-laravel/pull/2) | JackEllis | Partial ideas credited, full PR left open: APCu caches authenticated session/user rows for 300 seconds without immediate session/ban revocation invalidation. |
| [once-campfire-laravel #4](https://github.com/basecamp/once-campfire-laravel/pull/4) | SilentKernel | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-express #3](https://github.com/basecamp/once-campfire-express/pull/3) | pstachula-dev | Merged: independently checked; integrated production build included in the current comparison. |

## Revisions and attribution

| Implementation | Measured source revision | Production image | OCI revision label |
|---|---|---|---|
| Rails | [`27f5461`](https://github.com/basecamp/once-campfire/commit/27f5461067352e009e43b9b1780f802fd7d027a0) | `sha256:91570f23fcf793da69352fb9fad993f8cc0296cefa63eab176c20417a7a7e891` | `27f5461067352e009e43b9b1780f802fd7d027a0` |
| Django | [`bb73f7d`](https://github.com/basecamp/once-campfire-django/commit/bb73f7d94b798d7964538df36bc69b773c820289) | `sha256:d7ef39a2ddc852ba6f9a21a890dd67899f8c3e72b47f60ef6eacd68a4f38a9d3` | `bb73f7d94b798d7964538df36bc69b773c820289` |
| Laravel | [`1952aee`](https://github.com/basecamp/once-campfire-laravel/commit/1952aee93ab7a7bf83858ba3ea60646ecd804e88) | `sha256:d4adf7ca98aed7d72711cfece5e00594c6e1fa5fc79c3e57cbaacf2334bfcaa3` | `1952aee93ab7a7bf83858ba3ea60646ecd804e88` |
| Express | [`6465b2e`](https://github.com/basecamp/once-campfire-express/commit/6465b2e1b626406d68b2c7a0a963b5dc565c9a6e) | `sha256:b78ecb110476198fbf1ead7367f98877cfa1be1d18600c56c54607406978277d` | `6465b2e1b626406d68b2c7a0a963b5dc565c9a6e` |
| Elixir | [`db7958b`](https://github.com/basecamp/once-campfire-elixir/commit/db7958b4c39141c4574dabf21e3c2c301779891b) | `sha256:94408d30783dde6c14b95af4912fd4c43f06e26cb0f72d4b4fedb74c29988c16` | `db7958b4c39141c4574dabf21e3c2c301779891b` |
| Go | [`3396925`](https://github.com/basecamp/once-campfire-go/commit/33969250cefa99e3ba43d7431cbce1ab2cb14893) | `sha256:f1b429edfe0f7f66e012a316e931033726fe7bbd1a51ec31167b62bd2133ab9f` | `unlabelled` |
| Rust | [`743aa84`](https://github.com/basecamp/once-campfire-rust/commit/743aa8477ae99d8c0479fb5175dc0e466b6159d1) | `sha256:98d10429a9c977d6d00de6c8cc1cea2b773a427d4f74f8ceaf5034dd9eed5a95` | `unlabelled` |
| C | [`620a1f2`](https://github.com/basecamp/once-campfire-c/commit/620a1f2da8f74f10b8775d6c694a64dd3d24025f) | `sha256:9132cde0e9ccb1d20dae9842716cbec547f5c0f491f7826d8bcd3d6a9069a34f` | `620a1f2da8f74f10b8775d6c694a64dd3d24025f` |

The local receipts retain actual OCI labels as well as these image content IDs. Base-image labels
can identify the base runtime rather than the app. Benchmark-wrapper and documentation commits
may follow an image's app build without changing its application source; source equivalence was
checked for those cases. The C column is the native optimized build; Fil-C is tested separately
and its throughput is not inferred from the native result.

Contributor commits and partial-work co-author trailers preserve credit for sernle's full-sidebar
correction, Nick Potts's renderer incorporated by nijaru, Pasi Vuorio's literal-regex cache and Jack
Ellis's cache identity/token ideas. Daniel Collin ([emoon](https://github.com/emoon)) contributed
Rust database scheduling, rich-text rendering and cached-page gzip improvements in
[#43](https://github.com/basecamp/once-campfire-rust/pull/43). The public C fork retains
[mrsaraiva's original history](https://github.com/mrsaraiva/once-campfire-c).
