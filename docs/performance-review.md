# Campfire performance and verification, 2026-10-07

All eight implementations were measured with the public [shared verification harness](https://github.com/basecamp/once-campfire-verification).
Every measured response passed its content contract, and every acknowledged HTTP message
passed the exact persisted-write audit. Current results follow the completed functional checks below.
The run used harness revision [`d88b346`](https://github.com/basecamp/once-campfire-verification/commit/d88b3460620593bfb62e0bd1953b6462e01f230e).

24 performance pull requests are confirmed merged. The useful cache work from the separately
closed Laravel #1 is retained and credited. Merge author and committer are
`GPT on behalf of DHH <2741+dhh@users.noreply.github.com>`; original contributor commits remain intact.
39 proposals were reviewed across Rails, Rust, Go, Elixir, Laravel and Express; 24 merged and
15 remain unmerged, including two closed by their authors. Django and the
public [C fork](https://github.com/basecamp/once-campfire-c) are included in the current comparison.

## Current production HTTP results

Measured with 16 concurrent clients on an AMD Ryzen AI MAX+ 395 with 32 GB RAM,
with four hardware cores allocated to each app.

| HTTP workload (requests/sec) | Rails | [Django](https://github.com/basecamp/once-campfire-django) | [Laravel](https://github.com/basecamp/once-campfire-laravel) | [Express](https://github.com/basecamp/once-campfire-express) | [Elixir](https://github.com/basecamp/once-campfire-elixir) | [Go](https://github.com/basecamp/once-campfire-go) | [Rust](https://github.com/basecamp/once-campfire-rust) | [C](https://github.com/basecamp/once-campfire-c) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 710 | 414 | 1,696 | 42,481 | 1,126 | 52,512 | 105,909 | 137,505 |
| Messages page | 1,113 | 454 | 1,890 | 74,779 | 1,407 | 54,100 | 103,301 | 142,669 |
| Sidebar | 1,901 | 576 | 3,364 | 94,460 | 3,621 | 58,714 | 120,930 | 151,001 |
| Search | 1,332 | 549 | 2,615 | 83,493 | 2,127 | 60,444 | 121,502 | 148,766 |
| Post a message | 226 | 113 | 567 | 2,121 | 1,392 | 9,000 | 8,004 | 7,486 |

These are medians of 3 alternating rounds, with two-second warmups and
8-second samples. The applications run serially on CPUs 8–11; the generator
runs on separate physical cores 12–15. Builds, functional tests and other benchmarks were stopped.
Across the timed samples, **38,423,501 responses passed with zero request errors or invalid
responses**. The write audit verified **854,705 acknowledged warmup and timed writes**.
Peak recorded generator CPU was 195.0% of its four-core 400% capacity.

Laravel, Rails were remeasured serially after the latest integrations and review fixes. Complete three-round warmup and timed receipts replace their earlier samples; the other implementations retain their completed runs. Fixture, generator, response contracts and CPU allocations are unchanged.


Observed minimum–maximum rates across the measured rounds:

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 705–728 | 1,086–1,119 | 1,885–1,937 | 1,321–1,347 | 224–229 |
| Django | 414–415 | 453–456 | 574–577 | 548–550 | 113–113 |
| Laravel | 1,695–1,706 | 1,874–1,898 | 3,357–3,368 | 2,604–2,626 | 566–567 |
| Express | 42,318–43,072 | 74,227–75,179 | 91,871–95,295 | 67,252–84,619 | 2,091–2,127 |
| Elixir | 1,125–1,127 | 1,403–1,416 | 3,611–3,626 | 2,122–2,153 | 1,374–1,396 |
| Go | 52,312–52,562 | 53,882–54,181 | 58,246–59,135 | 59,950–61,194 | 8,978–9,044 |
| Rust | 105,102–106,662 | 102,522–103,589 | 119,955–121,370 | 120,342–121,944 | 7,978–8,039 |
| C | 137,462–139,842 | 142,501–145,474 | 148,180–151,318 | 147,754–148,959 | 7,473–7,520 |

## Before and after the architecture transfer

The earlier strict baseline and current run use the same fixture, response contracts,
16-client workload and four-core allocations. Each cell shows the earlier and current
median requests/sec, followed by the current/earlier ratio. Regressions are retained.
These compare complete configurations; they do not isolate a language or one cache change.

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 230 → 710 (3.09×) | 402 → 1,113 (2.77×) | 468 → 1,901 (4.06×) | 399 → 1,332 (3.34×) | 248 → 226 (0.91×) |
| Django | 62 → 414 (6.69×) | 70 → 454 (6.51×) | 229 → 576 (2.52×) | 118 → 549 (4.66×) | 112 → 113 (1.00×) |
| Laravel | 760 → 1,696 (2.23×) | 924 → 1,890 (2.05×) | 1,383 → 3,364 (2.43×) | 1,135 → 2,615 (2.30×) | 498 → 567 (1.14×) |
| Express | 2,622 → 42,481 (16.20×) | 3,245 → 74,779 (23.04×) | 34,938 → 94,460 (2.70×) | 6,613 → 83,493 (12.63×) | 2,088 → 2,121 (1.02×) |
| Elixir | 942 → 1,126 (1.20×) | 1,267 → 1,407 (1.11×) | 2,515 → 3,621 (1.44×) | 1,814 → 2,127 (1.17×) | 1,400 → 1,392 (0.99×) |
| Go | 31,673 → 52,512 (1.66×) | 30,746 → 54,100 (1.76×) | 18,586 → 58,714 (3.16×) | 29,765 → 60,444 (2.03×) | 9,073 → 9,000 (0.99×) |
| Rust | 35,484 → 105,909 (2.98×) | 40,674 → 103,301 (2.54×) | 34,479 → 120,930 (3.51×) | 34,432 → 121,502 (3.53×) | 8,998 → 8,004 (0.89×) |
| C | 141,834 → 137,505 (0.97×) | 151,564 → 142,669 (0.94×) | 159,850 → 151,001 (0.94×) | 155,456 → 148,766 (0.96×) | 7,460 → 7,486 (1.00×) |

The baseline used 3 rounds of 8-second samples and harness
[`f94e32e`](https://github.com/basecamp/once-campfire-verification/commit/f94e32e14e0de9ab1171409daeda94a505d22f9f).
Its frozen production image identities and complete receipts were checked locally.
The current harness adds the paced-writer profile; the ordinary route validation path is retained.
Earlier generator SHA-256: `fc43e2cdc920f8224abbe99ca03a8095f023758f4acc0825f7e1b66adccc3101`; current: `3decb2029d24156eba180ebfe9126f10d8dfce650743944660018c7fe82783b0`.

| Implementation | Earlier measured source revision |
|---|---|
| Rails | [`27f5461`](https://github.com/basecamp/once-campfire/commit/27f5461067352e009e43b9b1780f802fd7d027a0) |
| Django | [`bb73f7d`](https://github.com/basecamp/once-campfire-django/commit/bb73f7d94b798d7964538df36bc69b773c820289) |
| Laravel | [`1952aee`](https://github.com/basecamp/once-campfire-laravel/commit/1952aee93ab7a7bf83858ba3ea60646ecd804e88) |
| Express | [`6465b2e`](https://github.com/basecamp/once-campfire-express/commit/6465b2e1b626406d68b2c7a0a963b5dc565c9a6e) |
| Elixir | [`db7958b`](https://github.com/basecamp/once-campfire-elixir/commit/db7958b4c39141c4574dabf21e3c2c301779891b) |
| Go | [`3396925`](https://github.com/basecamp/once-campfire-go/commit/33969250cefa99e3ba43d7431cbce1ab2cb14893) |
| Rust | [`743aa84`](https://github.com/basecamp/once-campfire-rust/commit/743aa8477ae99d8c0479fb5175dc0e466b6159d1) |
| C | [`620a1f2`](https://github.com/basecamp/once-campfire-c/commit/620a1f2da8f74f10b8775d6c694a64dd3d24025f) |

## Reads with a paced message writer

This separate profile runs 16 read clients and one writer capped at 10 messages/sec,
with no catch-up bursts. Each route has a two-second warmup and
3 rounds of 8-second timed samples. Sources, production images,
fixture, generator and CPU allocations match the regular comparison above.

| Mixed HTTP workload (read requests/sec) | Rails | Django | Laravel | Express | Elixir | Go | Rust | C |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 70 | 55 | 1,059 | 36,823 | 264 | 48,272 | 101,129 | 128,459 |
| Messages page | 84 | 61 | 1,220 | 66,955 | 350 | 50,488 | 98,303 | 137,625 |
| Sidebar | 1,507 | 228 | 2,916 | 87,056 | 3,476 | 59,301 | 119,850 | 149,652 |
| Search | 517 | 104 | 2,185 | 76,414 | 1,317 | 59,163 | 117,670 | 146,149 |

Achieved writer cadence across the timed route samples:

| Implementation | Timed acknowledged writes | Writes per sample (min–max) | Writes/sec (min–max) |
|---|---:|---:|---:|
| Rails | 809 | 50–80 | 6.2–10.0 |
| Django | 817 | 62–80 | 7.7–10.0 |
| Laravel | 960 | 80–80 | 9.9–10.0 |
| Express | 960 | 80–80 | 9.9–9.9 |
| Elixir | 960 | 80–80 | 9.9–10.0 |
| Go | 960 | 80–80 | 9.9–9.9 |
| Rust | 960 | 80–80 | 9.9–9.9 |
| C | 960 | 80–80 | 9.9–9.9 |

**36,023,807 timed reads and 7,386 timed writes**
passed their response contracts, totaling **36,031,193 validated timed responses**
with zero request errors or invalid responses. The exact database/FTS audit verified
all **9,270 acknowledged writes**, including
1,884 warmup writes. Raw warmup and timed receipts
were reconciled with every application/round audit. The writer cap is a requested maximum,
rather than an assumption that every application achieved it.
Peak combined reader/writer generator CPU was 96.6% of its four-core 400% capacity.

## Response and persistence contracts

Each application gets a fresh copy of the complete Rails parity fixture, including real users,
memberships, rich text, uploads and variants. Push and webhook fixture URLs target a closed local
port. The fixture database hash is checked after the run; SQLite integrity is checked on each
application's disposable database. The repaired C seed no longer confuses user IDs with room IDs.

The 16 clients share one freshly authenticated fixture user/session within each application and
round. Repeated reads therefore exercise reuse for one viewer. Cache hit rates across thousands
of distinct viewers, and the resulting eviction pressure, are not measured here.

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
were not reused in this table. The before/after section uses a separately validated strict baseline with matching
fixtures, response contracts and hardware allocations.
HTTP rates do not measure WebSocket capacity or simultaneous people.

The client generator also has a four-core CPU limit; fast read workloads can approach that limit,
so their rates are not an independent measurement of maximum server capacity. Disposable SQLite
databases and storage in this run live on `/tmp` tmpfs. Exact transactional database/FTS persistence
was checked, but sustained NVMe write throughput and crash-safe disk durability were not measured.

## Completed functional checks

| Implementation | Completed checks | Explicit limits |
|---|---|---|
| Rails | 531 tests, 1,974 assertions; 27 Chromium system tests, 203 assertions; Rubocop and Brakeman. | Two unit skips for unsupported libvips loaders; no system-test skips. Initial browser timing failures were retained: the boost assertion passed focused and same-seed reruns, and the send helper now waits for the persisted response before leaving the job scope. The complete system suite passed at counter integration `fd49099`; later task-only repair has fresh native and production gates at `8de3e22`. |
| Django | 63 tests, Ruff and real Chromium functional flows. | Malformed rich-text and media byte parity are not established; live browser-provider Web Push requires external subscriptions. |
| Laravel | 71 tests, 945 assertions, Pint and real Chromium functional flows. | Exact rich-text parity, broader audio/video/PDF preview coverage, restore/upgrade and live browser-provider Web Push remain unverified in its contract ledger. |
| Express | 151 tests under Node and Bun, no skips, pinned formatter and real Chromium functional flows. | Public-site OpenGraph and live browser-provider Web Push remain unverified; malformed/legacy rich text outside the independent corpus can differ. |
| Elixir | Current 1,956-test native suite and strict compilation/formatting; one uninterrupted complete 65-gate parity run at the reviewed backend revision; rebuilt production asset manifest verified. | The complete parity run predates the latest browser fixes; current regression checks supplement it. Verification used pinned OTP 29 in a disposable checkout; no production cutover is claimed. |
| Go | Full race/vet/assets/WebSocket-fork suite; real Chromium; production backup/restore/restart; real Go/Rust cookie and message/FTS interoperability; real ACME issuance and cached HTTPS restart with the CA offline. | Strict byte-for-byte HTML/network parity remains incomplete, as documented by the port. |
| Rust | Current seed-required workspace: 794 passed; clippy with warnings denied, formatting and unused-dependency checks clean. Earlier complete five-seed browser inventories: 956 cells, 954 passes, zero failures, flakes or errors; rebuilt embedded frontend verified. | 11 existing timing/reference-export/doc cases explicitly ignored; no missing-seed skips. Earlier browser inventories predate the latest frontend fixes; fresh shared browser regressions supplement them. Two documented manifest JSON escaping allowances. |
| C | Current 1,676-test native suite; current affected sanitizer subset (135 tests); earlier complete 1,673-test ASan/UBSan/LSan and 1,673-test Fil-C runs; threaded TSan subset; pinned media vectors; repeated four-browser HTTPS flows with real PNG/video uploads. | Complete sanitizer/Fil-C receipts predate the latest view repairs; the affected sanitizer subset is checked separately. Granted real-browser Web Push delivery is not covered in the headless environment; denied-permission UI, native encryption and local delivery were checked. Existing-install upgrades and ACME remain outside this port's contract. |

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

Claude (Opus 5.5) reviewed the prepared changes from all eight implementations and the
follow-up Rails, Laravel and Rust fixes. Confirmed issues were reproduced locally: native
fallback fragments could outlive foreign writes, Rust detached presentation could capture its
generation after reading old data, and Rails instrumented fragment keys exposed the raw session
credential. The fixes, source-validated non-blockers and review receipts were retained locally.
A final approved incremental review covered the latest Rails #337 and Laravel #2 integrations.
Its concrete follow-up findings were reproduced and corrected: SQLite schema/migration tasks
now repair counters in the selected database on every invocation; Laravel writer waits respect
the configured busy timeout, lock paths use the opened canonical database, and raw transactions
retain their lock, bypass committed caches and roll back at the Octane request boundary. Cookie
payload comparison is strict. Speculative findings were checked against actual framework source;
startup still fails closed if counter repair cannot take the SQLite write lock.
The paced-writer profile addresses the review's cache-churn concern; correctness checks do not
establish a speedup under every workload.

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

All eight now have bounded authenticated response caching for the room, message, sidebar and
search reads. Each cache defaults to a 64 MiB accounting budget, including its entry accounting;
this is a cache budget, not total process memory, and separate workers can each have a cache.
`CAMPFIRE_RESPONSE_CACHE_MB=0` disables whole-response reuse in the seven new ports; Laravel
also disables its new fragment reuse. C uses its existing `CF_CACHE_BYTES` configuration. Hits retain fresh authentication and authorization checks and observe local and
external SQLite commits. Lookup and admission guard against commits racing authentication or
rendering; request variants retain the native origin, session, format and rendering contracts.
Nested fragment caches also account for foreign updates without a timestamp change.
Rails gives native ERB collection and Jbuilder fragments a separate 64 MiB memory budget per
worker, namespaces them with the pre-authentication snapshot and rendering context, and keeps
shared rate-limit stores unchanged. Sixteen fixed locks collapse concurrent cold page renders.
Rails bot pagination uses SQLite-maintained counters; ordinary foreign and bulk writes are
covered by insert/delete/room-move triggers. Migration backfill, missing-trigger repair and
replacement are atomic, and schema/migration tasks restore counters before later task steps.
External REPLACE writers must enable recursive_triggers so SQLite fires the corresponding delete
trigger; migrations rebuilding messages must repair counters before reading them inside that
migration. The shared HTTP table does not quantify the separate bot-count query improvement.
Laravel shares its 64 MiB budget between pages and token-neutral native fragments, with snapshot
capture on every route. Rust captures detached presentation snapshots before their database
reads; detached Rails renderers bypass fragment reuse because they do not run those callbacks.

Rails, Django and Elixir keep fresh per-request CSRF masking by caching an identity body and
finishing the token and compression work per request. Laravel preserves its stable session CSRF
policy; Go, Rust, Express and C preserve their existing forgery-protection policy and reuse complete
encoded responses where that is compatible. Thus the read hot paths have different remaining
compression work. The normal results include warm reads; the separate mixed profile, when
supplied above, measures cache invalidation and reads alongside actual message writes.

The table compares complete production configurations, rather than isolating language speed.
Rails uses Puma and Redis/Resque; Elixir retains Redis/Resque compatibility; Django and Express
use affinity-sized HTTP workers in the measured production configuration; Laravel uses persistent
Octane workers; Go, Rust and C run their native servers. Exact worker topology and runtime
settings are retained with the local metadata. Compatible query, cache, permission and frontend
lessons were transferred; this does not claim that every runtime architecture is interchangeable.

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
| [once-campfire #321](https://github.com/basecamp/once-campfire/pull/321) | thomasklemm | Closed by its author in favor of #337; the successor's SQLite-trigger maintenance and independent cross-writer controls passed before merging. |
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
| [once-campfire-go #5](https://github.com/basecamp/once-campfire-go/pull/5) | kidandcat | Latest `63dd3a5` adds a pinned data_version observer and foreign-connection tests, fixing the original local-only objection. Source review still finds stale nested message HTML after unversioned foreign edits and transaction bookkeeping that can absorb a foreign commit without invalidating warm windows. Its advertised throughput is not independently verified. |
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
| [once-campfire-laravel #2](https://github.com/basecamp/once-campfire-laravel/pull/2) | JackEllis | Merged at `ffb6db2`, retaining Jack Ellis's contributor history and credit. Independent 64-test native and production checks passed; formatted sound commands and failed-COMMIT lock handling were corrected. Latest integrated source was remeasured with validated responses and exact write audits; advertised branch throughput is not assumed. |
| [once-campfire-laravel #4](https://github.com/basecamp/once-campfire-laravel/pull/4) | SilentKernel | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire-express #3](https://github.com/basecamp/once-campfire-express/pull/3) | pstachula-dev | Merged: independently checked; integrated production build included in the current comparison. |
| [once-campfire #337](https://github.com/basecamp/once-campfire/pull/337) | thomasklemm | Merged at `fd49099`, preserving Thomas Klemm's original contributor history. Latest `b4ab2df` fixes atomic trigger replacement and missing-trigger repair. Full native and browser suites plus independent legacy-schema migration, foreign/bulk/rollback/repair controls passed; populated bot pagination returns exact IDs/totals without a message COUNT query. The shared five-route table does not measure the bot pagination benefit. |

Constructive explanations were posted on all 17 proposals that were initially unmerged, with
source-review findings distinguished from independently validated changes.

| Initially unmerged proposal | Published explanation |
|---|---|
| [once-campfire #164](https://github.com/basecamp/once-campfire/pull/164) | [Review comment](https://github.com/basecamp/once-campfire/pull/164#issuecomment-6043359249) |
| [once-campfire #319](https://github.com/basecamp/once-campfire/pull/319) | [Review comment](https://github.com/basecamp/once-campfire/pull/319#issuecomment-6043360009) |
| [once-campfire #321](https://github.com/basecamp/once-campfire/pull/321) | [Review comment](https://github.com/basecamp/once-campfire/pull/321#issuecomment-6043360904) |
| [once-campfire #337](https://github.com/basecamp/once-campfire/pull/337) | [Review comment](https://github.com/basecamp/once-campfire/pull/337#issuecomment-6047221603) |
| [once-campfire-elixir #1](https://github.com/basecamp/once-campfire-elixir/pull/1) | [Review comment](https://github.com/basecamp/once-campfire-elixir/pull/1#issuecomment-6043362105) |
| [once-campfire-elixir #2](https://github.com/basecamp/once-campfire-elixir/pull/2) | [Review comment](https://github.com/basecamp/once-campfire-elixir/pull/2#issuecomment-6043362574) |
| [once-campfire-elixir #3](https://github.com/basecamp/once-campfire-elixir/pull/3) | [Review comment](https://github.com/basecamp/once-campfire-elixir/pull/3#issuecomment-6043363360) |
| [once-campfire-elixir #5](https://github.com/basecamp/once-campfire-elixir/pull/5) | [Review comment](https://github.com/basecamp/once-campfire-elixir/pull/5#issuecomment-6043364117) |
| [once-campfire-elixir #6](https://github.com/basecamp/once-campfire-elixir/pull/6) | [Review comment](https://github.com/basecamp/once-campfire-elixir/pull/6#issuecomment-6043365012) |
| [once-campfire-go #2](https://github.com/basecamp/once-campfire-go/pull/2) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/2#issuecomment-6043365835) |
| [once-campfire-go #4](https://github.com/basecamp/once-campfire-go/pull/4) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/4#issuecomment-6043366407) |
| [once-campfire-go #5](https://github.com/basecamp/once-campfire-go/pull/5) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/5#issuecomment-6047714954) |
| [once-campfire-go #6](https://github.com/basecamp/once-campfire-go/pull/6) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/6#issuecomment-6043367657) |
| [once-campfire-go #7](https://github.com/basecamp/once-campfire-go/pull/7) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/7#issuecomment-6043368328) |
| [once-campfire-go #8](https://github.com/basecamp/once-campfire-go/pull/8) | [Review comment](https://github.com/basecamp/once-campfire-go/pull/8#issuecomment-6043369224) |
| [once-campfire-laravel #1](https://github.com/basecamp/once-campfire-laravel/pull/1) | [Review comment](https://github.com/basecamp/once-campfire-laravel/pull/1#issuecomment-6043369923) |
| [once-campfire-laravel #2](https://github.com/basecamp/once-campfire-laravel/pull/2) | [Review comment](https://github.com/basecamp/once-campfire-laravel/pull/2#issuecomment-6046284431) |

## Revisions and attribution

| Implementation | Measured source revision | Production image | OCI revision label |
|---|---|---|---|
| Rails | [`8de3e22`](https://github.com/basecamp/once-campfire/commit/8de3e22ab43120d04af0de739c92ee865f079cb4) | `sha256:1b0eba554b63423b17efdf25010f5f6a7dadc5ad1032cdb12022567acd96baa0` | `8de3e22ab43120d04af0de739c92ee865f079cb4` |
| Django | [`5834bc2`](https://github.com/basecamp/once-campfire-django/commit/5834bc2c6441618c70a63ca39fd1249c1bf9169b) | `sha256:78409f0bdf6b27c003bf3cb40e05e04b961ae4ce23fef7efe6df14d5a5634ad5` | `5834bc2c6441618c70a63ca39fd1249c1bf9169b` |
| Laravel | [`f054cd6`](https://github.com/basecamp/once-campfire-laravel/commit/f054cd6a70e80618714b7d47ba5b9db48bed1ff3) | `sha256:695b8882468a64f8585baba006d51c5fd9689e00f98d52c325401ec33a5a6765` | `f054cd6a70e80618714b7d47ba5b9db48bed1ff3` |
| Express | [`f0a7d96`](https://github.com/basecamp/once-campfire-express/commit/f0a7d96565c0bbd35772d5bd962b7c6625bbdae1) | `sha256:8f0e866fd37862647dcc57d715c8a5241894748c4387e8a0beccdfcb45c07e8e` | `f0a7d96565c0bbd35772d5bd962b7c6625bbdae1` |
| Elixir | [`931ed55`](https://github.com/basecamp/once-campfire-elixir/commit/931ed556644b7848c72f732e0e603dd34760d29f) | `sha256:4f58f03d7ac9a03fd2c909c6d9b4f142dc410b75079c87dee98a8d621a0f1e68` | `931ed556644b7848c72f732e0e603dd34760d29f` |
| Go | [`6eb12e8`](https://github.com/basecamp/once-campfire-go/commit/6eb12e8599cafeb3ff8229e237b4efdcf09fee24) | `sha256:e21db245a7523384fbb7ee7f84352029c396b5a26b2efa6a8bfb8ea7a9448419` | `unlabelled` |
| Rust | [`d54b303`](https://github.com/basecamp/once-campfire-rust/commit/d54b3035a4ab39c2d9b747c1a9698d6052965367) | `sha256:a368be61a026aa8f78d04019beb8bfb2870db53bf299ac2fa7cd116b6268d87f` | `unlabelled` |
| C | [`277fee6`](https://github.com/basecamp/once-campfire-c/commit/277fee6ced976877ad9e08f3383c610b6a765577) | `sha256:41da1ded88b26948a85780afc7ec8dc4bcc6082f8b7c70e8dfd26f9c94d2d404` | `277fee6ced976877ad9e08f3383c610b6a765577` |

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

## Matched paced-writer controls

The three frameworks with substantial native fragment reuse were also measured at their
frozen earlier revisions using the same fixture, harness, allocations and paced-writer workload.
Each cell shows earlier → current read requests/sec and the current/earlier ratio.

| Implementation | Room page | Messages page | Sidebar | Search |
|---|---:|---:|---:|---:|
| Rails | 205 → 70 (0.34×) | 369 → 84 (0.23×) | 425 → 1,507 (3.55×) | 359 → 517 (1.44×) |
| Laravel | 728 → 1,059 (1.45×) | 892 → 1,220 (1.37×) | 1,361 → 2,916 (2.14×) | 1,092 → 2,185 (2.00×) |
| Elixir | 931 → 264 (0.28×) | 1,252 → 350 (0.28×) | 2,495 → 3,476 (1.39×) | 1,796 → 1,317 (0.73×) |

| Implementation | Earlier writer requests/sec (min–max) | Current writer requests/sec (min–max) | Earlier source |
|---|---:|---:|---|
| Rails | 9.4–10.0 | 6.2–10.0 | [`27f5461`](https://github.com/basecamp/once-campfire/commit/27f5461067352e009e43b9b1780f802fd7d027a0) |
| Laravel | 9.9–10.0 | 9.9–10.0 | [`1952aee`](https://github.com/basecamp/once-campfire-laravel/commit/1952aee93ab7a7bf83858ba3ea60646ecd804e88) |
| Elixir | 9.9–10.0 | 9.9–10.0 | [`db7958b`](https://github.com/basecamp/once-campfire-elixir/commit/db7958b4c39141c4574dabf21e3c2c301779891b) |

The earlier controls validated 286,392 timed reads and 2,869 timed writes
with zero errors or invalid responses, and audited all 3,582 warmup and timed writes.
The cadence ranges are reported because equal writer caps do not guarantee equal achieved write rates.
These are complete implementation configurations, rather than an isolated cache microbenchmark.
