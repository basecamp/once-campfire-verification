# Campfire performance and verification, 2026-10-08

The implementations retain the Campfire design and installation contracts while using native backends, Fetch Metadata for browser writes, fresh authorization, and bounded caches of complete HTML/gzip responses. Rails now runs Ruby 4.0.7 with Puma 8.0.2; its rendering and message-write paths have also been simplified. Express retains Express and Node/Bun support while reducing redundant message-creation work. Laravel shortens authenticated read/session paths and creation/broadcast work while preserving atomic writes and fresh permissions.

## Current production HTTP results

Measured with 16 concurrent clients on an AMD Ryzen AI MAX+ 395 with 32 GB RAM, with four hardware cores allocated to each app.

| HTTP workload (requests/sec) | Rails | [Django](https://github.com/basecamp/once-campfire-django) | [Laravel](https://github.com/basecamp/once-campfire-laravel) | [Express](https://github.com/basecamp/once-campfire-express) | [Elixir](https://github.com/basecamp/once-campfire-elixir) | [Go](https://github.com/basecamp/once-campfire-go) | [Rust](https://github.com/basecamp/once-campfire-rust) | [C](https://github.com/basecamp/once-campfire-c) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 4,121 | 478 | 3,872 | 42,636 | 5,350 | 53,060 | 106,494 | 137,524 |
| Messages page | 4,026 | 486 | 3,995 | 74,362 | 5,712 | 54,800 | 102,697 | 144,642 |
| Sidebar | 4,195 | 601 | 4,493 | 94,329 | 5,949 | 59,144 | 120,294 | 152,002 |
| Search | 4,098 | 594 | 4,172 | 84,665 | 5,848 | 60,509 | 121,378 | 149,487 |
| Post a message | 323 | 112 | 794 | 2,155 | 1,278 | 9,021 | 8,037 | 7,530 |

Rails was remeasured on October 8, 2026; Express and Laravel were measured later that day after their selected changes. Each session uses three alternating rounds, two seconds of warmup and eight seconds per sample. The five untouched implementations (Django, Elixir, Go, Rust and C) retain exactly their independently verified figures from the [previous published comparison](https://github.com/basecamp/once-campfire-verification/blob/70e612c1770be352e94d8ccb74184dce74ea6d33/docs/performance-review.md); they were not rerun for this update. All sessions use the same machine, CPU allocation, fixture, 16-client contracts and gzip. This current configuration table combines those sessions and does not imply a fresh simultaneous eight-build comparison.

All applications run serially on CPUs 8–11; the generator uses 12–15. Builds, tests and other benchmarks stop during timing. The completed Rails/unchanged-Laravel control session validated 746,954 timed responses and audited 26,081 exact acknowledged writes including warmup, with zero errors or invalid responses. Its 60 raw samples reconciled and peak generator CPU was 9.134% of four-core 400% capacity. The new Express/Laravel session validated 7,416,673 timed responses and audited 85,582 acknowledged writes, with zero request errors or invalid responses; its peak generator CPU was 59.960%. All 60 raw warmup/timed samples reconciled against per-application/round exact database/FTS audits.

Observed minimum–maximum requests/sec:

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 4,100–4,133 | 3,940–4,180 | 4,156–4,476 | 4,014–4,321 | 319–328 |
| Django | 478–484 | 485–490 | 599–605 | 593–599 | 112–114 |
| Laravel | 3,847–3,919 | 3,970–4,031 | 4,464–4,522 | 4,146–4,201 | 794–797 |
| Express | 42,203–43,372 | 73,931–74,879 | 92,055–94,338 | 66,159–85,572 | 2,078–2,176 |
| Elixir | 5,342–5,360 | 5,686–5,717 | 5,923–5,982 | 5,821–5,937 | 1,278–1,298 |
| Go | 52,795–53,610 | 54,720–54,811 | 58,585–60,178 | 60,088–61,007 | 9,002–9,098 |
| Rust | 105,504–107,910 | 102,687–103,274 | 119,011–121,793 | 120,984–124,036 | 7,988–8,038 |
| C | 136,404–139,097 | 143,942–149,113 | 151,566–152,698 | 148,288–151,008 | 7,512–7,556 |

Rails uses four Puma workers with one request thread each and retains three Resque workers. The cache budgets remain per process, so this worker layout changes aggregate memory and cache capacity. Laravel retains FrankenPHP/Octane, its ReactPHP Cable server, and leased auxiliary SQLite jobs. The prior published Rails topology and current topology are part of the configuration comparison; the result does not isolate a language, a single optimization, Ruby, or Puma.

## Reads with a paced message writer

This separate profile uses 16 read clients plus one writer capped at ten messages/sec, without catch-up bursts. It has the same warmup, three eight-second samples, sources, production images, fixture and CPU allocations as the normal profile. Rails retains its completed current-configuration measurement, Express and Laravel are newly measured, and the five other columns retain their earlier published profile.

| Mixed HTTP workload (read requests/sec) | Rails | Django | Laravel | Express | Elixir | Go | Rust | C |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 2,190 | 58 | 2,277 | 37,831 | 1,438 | 48,465 | 99,660 | 130,733 |
| Messages page | 2,709 | 61 | 2,472 | 66,668 | 1,694 | 49,902 | 98,055 | 137,520 |
| Sidebar | 3,663 | 236 | 3,896 | 88,594 | 5,602 | 59,331 | 117,710 | 149,740 |
| Search | 3,334 | 107 | 3,371 | 76,695 | 4,009 | 58,944 | 117,919 | 145,166 |

The writer posts to HQ while room/message reads target Watercooler. Global SQLite epochs invalidate these read pages even when the write belongs to another room. Sidebar and search retain their normal scopes. This measures cross-room invalidation and concurrent writes; it does not model every reader following one actively written room or WebSocket fanout.

| Implementation | Timed acknowledged writes | Writes per sample (min–max) | Writes/sec (min–max) |
|---|---:|---:|---:|
| Rails | 960 | 80–80 | 9.9–10.0 |
| Django | 830 | 65–79 | 8.0–9.9 |
| Laravel | 960 | 80–80 | 9.9–10.0 |
| Express | 960 | 80–80 | 9.9–9.9 |
| Elixir | 960 | 80–80 | 9.9–10.0 |
| Go | 960 | 80–80 | 9.9–9.9 |
| Rust | 960 | 80–80 | 9.9–9.9 |
| C | 960 | 80–80 | 9.9–9.9 |

The completed Rails/unchanged-Laravel control session validated 523,955 timed reads and 1,920 timed writes with zero errors or invalid responses. Its database/FTS audit reconciled 2,388 acknowledged writes, including 468 warmup writes, across 48 raw samples; peak combined generator CPU was 8.225%. The new Express/Laravel session validated 6,748,264 timed reads and 1,920 timed writes, and audited 2,400 acknowledged writes including 480 warmup writes, with zero request errors or invalid responses. Peak combined generator CPU was 56.294% of its four-core 400% capacity. The writer cap is a requested maximum; achieved cadence is reported above. All 48 raw warmup/timed samples reconciled against exact database/FTS audits. Both sources were clean and matched the same image revision labels used in the normal session.

Observed mixed read minimum–maximum requests/sec:

| Implementation | Room page | Messages page | Sidebar | Search |
|---|---:|---:|---:|---:|
| Rails | 2,180–2,276 | 2,488–2,735 | 3,656–3,702 | 3,126–3,368 |
| Django | 55–58 | 59–61 | 216–237 | 105–108 |
| Laravel | 2,243–2,304 | 2,406–2,481 | 3,754–4,053 | 3,303–3,388 |
| Express | 36,645–39,341 | 64,028–67,281 | 87,896–88,957 | 75,883–77,602 |
| Elixir | 1,378–1,526 | 1,650–1,713 | 5,554–5,610 | 3,930–4,031 |
| Go | 48,054–48,680 | 49,793–50,521 | 58,774–59,661 | 58,688–59,050 |
| Rust | 99,370–100,957 | 98,012–98,126 | 117,399–119,116 | 116,797–118,859 |
| C | 130,011–130,879 | 137,449–137,649 | 148,506–151,328 | 145,105–145,584 |

## Compared with the previous published configuration

The five unchanged headline and mixed columns are retained, so no new before/after claim is made for them. The complete older eight-implementation comparison, historical architecture-transfer figures and earlier attribution remain in the [pinned published report](https://github.com/basecamp/once-campfire-verification/blob/70e612c1770be352e94d8ccb74184dce74ea6d33/docs/performance-review.md). The figures below compare complete Rails/Express/Laravel configurations with that report and retain regressions.

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 2,063 → 4,121 (2.00×) | 2,063 → 4,026 (1.95×) | 2,545 → 4,195 (1.65×) | 2,528 → 4,098 (1.62×) | 234 → 323 (1.38×) |
| Laravel | 3,038 → 3,872 (1.27×) | 3,081 → 3,995 (1.30×) | 3,832 → 4,493 (1.17×) | 3,710 → 4,172 (1.12×) | 577 → 794 (1.38×) |
| Express | 43,925 → 42,636 (0.97×) | 74,176 → 74,362 (1.00×) | 94,322 → 94,329 (1.00×) | 82,937 → 84,665 (1.02×) | 2,098 → 2,155 (1.03×) |

| Mixed reads | Room page | Messages page | Sidebar | Search |
|---|---:|---:|---:|---:|
| Rails | 219 → 2,190 (10.00×) | 221 → 2,709 (12.26×) | 2,046 → 3,663 (1.79×) | 1,009 → 3,334 (3.30×) |
| Laravel | 1,758 → 2,277 (1.30×) | 1,862 → 2,472 (1.33×) | 3,377 → 3,896 (1.15×) | 2,973 → 3,371 (1.13×) |
| Express | 37,455 → 37,831 (1.01×) | 65,935 → 66,668 (1.01×) | 87,999 → 88,594 (1.01×) | 75,347 → 76,695 (1.02×) |

The prior Rails mixed profile acknowledged 803 timed writes (47–80 per sample, 5.7–10.0/sec), compared with the current 960 (80 per sample, 9.9–10.0/sec). Laravel acknowledged 960 in the completed control profile; its new cadence is shown above. Different achieved cadence and cache churn prevent treating the mixed ratios as an isolated cache benchmark. The unmodified published ranges and writer rates are preserved above for the five untouched implementations.

## What changed

Rails reuses request-local record snapshots and keys native fragments from presentation content. It avoids eager joins on cache hits, renders each completed response once, uses literal HTML for quick boosts, and avoids duplicating the parsed DOM when converting attachmentless rich text to plain text. Native transaction detection uses a short pool checkout rather than retaining a connection on the cache-hit path; fixture-pinned transactions still bypass caching. These changes retain fresh authorization, request-origin namespaces, SQLite epoch checks and conditional-response semantics.

Message creation now saves its Action Text and attachment metadata before writing FTS and unread state within the native database transaction. Enqueueing and network work remain after commit; update/destroy indexing retains its native behavior. Existing memberships already unread are not rewritten. SQLite checkpointing runs in the background with one elected owner, failover and fork-safe cleanup. Native regressions check rollback and exactly-once outcomes, rather than inferring persistence from a successful HTTP acknowledgment. The atomic-creation change consolidates the work into one commit and protects correctness; its exploratory posting control was effectively flat (322.8 → 323.4 requests/sec), so no independent speed gain is attributed to atomicity.

Whole-page and fragment budgets remain separate, bounded per-worker caches. Complete tokenless identity/gzip bodies are reusable; live cookies and authorization remain per request. Express keeps its existing framework, authorization and SQLite epoch caches. It takes the new rich-text id from INSERT, directly inserts a new AUTOINCREMENT message's FTS row, and reuses the already-rendered broadcast fragment for the poster response. Its focused control reduces the ordinary creation path from ten to eight SQL statements and renders that message once; rollback, upload and delivery ordering tests preserve native semantics. These are two small runtime-file changes extracted from the larger proposal.

Laravel checks membership afresh on every cache hit using a narrow existence query and loads the Room model only on a miss. Unchanged native session reads avoid rewriting the session file/cookie until the half-lifetime refresh boundary, and the permanent last-room cookie changes only when the room changes. Creation retains already-known relationships, directly inserts new rich text/search rows, and batches ordered room/unread outbox appends under one lock. Integration corrections keep message, rich text, FTS and unread updates atomic, resolve SGID-backed mention plain text after taking the write lock, and transfer Rails #336's first-unread policy for shared rooms. Existing pre-authentication snapshots, admission checks, bounds, gzip and foreign-writer controls remain.

## Source, images and checks

| Implementation | Measured source | Frozen image | Native checks |
|---|---|---|---|
| Rails | [`7331d3a`](https://github.com/basecamp/once-campfire/commit/7331d3a9467f90fda5812674eb9c7dd5d2c8deb4) | `sha256:786719c80b6009125d70454ed0518d8d5f8006d3a5abfbd882b2810dd56afa13` | 571 / 2,233 assertions; system 27 / 203 |
| Django | [`04f1f27`](https://github.com/basecamp/once-campfire-django/commit/04f1f273f43ab3f0c66565082ca3fd31eb994ce6) | `sha256:1d1a7d4226d7881d418054dbde2c4ad258ac604faee01763eb91846af506c843` | 64 |
| Laravel | [`3b4a889`](https://github.com/basecamp/once-campfire-laravel/commit/3b4a889a13f4bf151d650f3887219f807607a642) | `sha256:8ad05414ed618f6f985d014b55aeef52a9916b1d9208c8f6dcf23785d11e4e8d` | 80 / 1,005 assertions; Pint 80 files |
| Express | [`1fc0949`](https://github.com/basecamp/once-campfire-express/commit/1fc09490ae0958ede218d62297c20dbb9e0557cd) | `sha256:cd32e2d25410575ee206afc5f3cd46491bc2b26a6034da86cd6049353256df9a` | 158 Node + 158 Bun; formatter/assets |
| Elixir | [`fa7f2ec`](https://github.com/basecamp/once-campfire-elixir/commit/fa7f2ecf89d560bb64a1c215e5693fa285c02705) | `sha256:947432c2355e89bba7dd660e3a5a12fc170d16d5c616d44674a49d8a8943225b` | 1961 |
| Go | [`7fc7412`](https://github.com/basecamp/once-campfire-go/commit/7fc7412db06e1f3520d4377d5fdc3855f65abfb1) | `sha256:03dd26fb3aff9ed445a8c28833aa0eb7c060ae2ac1c29c9d4ba089e0dbb1dc06` | Full race suite and vet |
| Rust | [`f9dca47`](https://github.com/basecamp/once-campfire-rust/commit/f9dca47e592fdcd23a75e4c8b6b97f5d452ccdc1) | `sha256:a368be61a026aa8f78d04019beb8bfb2870db53bf299ac2fa7cd116b6268d87f` | 795 passed; 11 existing ignores |
| C | [`cd0cbe4`](https://github.com/basecamp/once-campfire-c/commit/cd0cbe4b24bcee0aa6ad75e6bc29d4e773628332) | `sha256:41da1ded88b26948a85780afc7ec8dc4bcc6082f8b7c70e8dfd26f9c94d2d404` | 1676 full historical native cases; current Fetch Metadata affected subset is separately recorded |

The five untouched rows identify the exact source and image that produced their published timings. Later test/documentation-only changes, including Rust #42, do not replace those measured identifiers. Fresh current checks supplement earlier evidence rather than retroactively relabeling an old image.

New measured Rails source: `7331d3a9467f90fda5812674eb9c7dd5d2c8deb4`; frozen image: `sha256:786719c80b6009125d70454ed0518d8d5f8006d3a5abfbd882b2810dd56afa13`. New Express source: `1fc09490ae0958ede218d62297c20dbb9e0557cd`; frozen image: `sha256:cd32e2d25410575ee206afc5f3cd46491bc2b26a6034da86cd6049353256df9a`. New Laravel source: `3b4a889a13f4bf151d650f3887219f807607a642`; frozen image: `sha256:8ad05414ed618f6f985d014b55aeef52a9916b1d9208c8f6dcf23785d11e4e8d`. The image-label/runtime byte audits and fresh production receipts are matched to these exact inputs. Published Rails [`2e89e08`](https://github.com/basecamp/once-campfire/commit/2e89e0800395f11834a49b82caab5bfeec157fb4) preserves upstream contributor history and has the identical tree `66d345e1a5822153e1530e10f69bdf1a14a2c877` to the measured `7331d3a` source. Its 568-file runtime byte audit passed. The selected Express 620-file and Laravel 420-file runtime byte audits passed. New Express/Laravel measured harness: [`70e612c`](https://github.com/basecamp/once-campfire-verification/commit/70e612c1770be352e94d8ccb74184dce74ea6d33). The completed normal session records both clean exact source heads and their matching immutable image revision labels; the mixed profile uses the same frozen inputs.

New measured harness: [`70e612c`](https://github.com/basecamp/once-campfire-verification/commit/70e612c1770be352e94d8ccb74184dce74ea6d33); generator SHA-256: `26ac5ed0a67b04867583c3745da4c266b417750fb2309e47bc3357c206735025`. The five retained measurements used harness [`b95d5a7`](https://github.com/basecamp/once-campfire-verification/commit/b95d5a7bfbc8ce1ae03d01d293d19dd719f0d86d) and generator `26ac5ed0a67b04867583c3745da4c266b417750fb2309e47bc3357c206735025`.

Seed SHA-256: `036edd6a07815bbddbfe60949d065caf777cee6f0c1a96e286e08c16e584c60c`. The completed Rails session uses the same generator hash as the five retained measurements; new Express/Laravel generator SHA-256: `26ac5ed0a67b04867583c3745da4c266b417750fb2309e47bc3357c206735025`. The additional harness changes are optional profiles/adapters and reporting/tests; the timed HTTP response and persisted-write contracts remain unchanged.

Rails has 571 native tests / 2,233 assertions and two unsupported-loader skips (`matload`, `niftiload`). Its 27 Chromium tests / 203 assertions pass with no skips. RuboCop, Herb and Brakeman passed. The corrected libvips tests independently invoke forbidden loaders, rather than confusing loader discovery with loader execution. Ruby 4's separately built native gems and exporter archive were tested; no Ruby 3.4 ABI bundle was reused.

Laravel's current full suite has 80 tests / 1,005 assertions with zero skips, plus Pint 80 files and fresh production browser checks. Express passes 158 tests under Node and 158 under Bun with zero skips; the table measures Node, with no new Bun speed claim. Final Rails Chromium replay passed 27 tests / 203 assertions with no failures, errors or skips. Published Rails [CI](https://github.com/basecamp/once-campfire/actions/runs/37770982879), [image build](https://github.com/basecamp/once-campfire/actions/runs/37770982884) and [push checks](https://github.com/basecamp/once-campfire/actions/runs/37770982474) all passed at the exact published head.

Fresh strict production gates cover setup, live messaging, editing, stored-markup safety, profiles/accounts, bots, styles, session transfers, joining and direct pings. Dedicated security/cache controls check Fetch Metadata, legacy installation cookies, signed-upload/session capabilities, gzip quality negotiation, cache64/0, no-timestamp foreign SQL body/creator/boost edits, stale conditional requests and revoked sessions/memberships. Migration controls validate historical counterless fixtures, trigger backfill and counter integrity after live flows. Final Express/Laravel production browser and signed-capability security controls passed with cache 64 and 0 against their selected images. Published [Express checks](https://github.com/basecamp/once-campfire-express/actions/runs/37783279609), [Express CI](https://github.com/basecamp/once-campfire-express/actions/runs/37783278323) and [Laravel CI](https://github.com/basecamp/once-campfire-laravel/actions/runs/37783283932) passed at those exact source heads. The final Rails cache-disabled controls also passed; counter integrity remained exact after browser and foreign-SQL controls.

Earlier failed attempts remain in local receipts. The initial four-worker cache-hit candidate exhausted the database pool; the final transaction guard was tested against both normal leasing and fixture-pinned transactions. A Puma 8 production browser attempt timed out waiting for a RoomMessages subscription while concurrent Chromium suites ran; quiet reruns passed against the same source/image. Successful final checks do not erase these attempts or establish that browsers never flake.

Django and Elixir published full suites have no skips. Rust has 11 existing ignored timing/reference-export/doc cases and no missing-seed skips. C's full 1,676 native cases and current affected subset remain the evidence identified in the prior report. Historical Elixir parity, Rust multi-seed parity and C sanitizer/Fil-C inventories predate these latest changes; they were not all rerun. Each port retains its documented rich-text, media, push and upgrade limits.

## Reproduce

Build each chosen revision's production image first, then set `RAILS_IMAGE`, `EXPRESS_IMAGE`, `LARAVEL_IMAGE` and the other image overrides. Retain the reported topology and cache budgets in your benchmark environment. The runner defaults to three rounds, 16 clients, eight-second samples and four CPU cores per app. For the exact normal and mixed route sets:

```sh
npm ci
npx playwright install chromium
bin/check
bin/seed
cargo build --release --locked --manifest-path loadgen/Cargo.toml
export RAILS_BENCH_ENV='{"WEB_CONCURRENCY":"4","RAILS_MAX_THREADS":"1","RAILS_MIN_THREADS":"1","JOB_CONCURRENCY":"3"}'
bin/benchmark --apps rails,express,laravel --rounds 3 --duration 8 --concurrencies 16 \
  --cpus 8-11 --client-cpus 12-15 \
  --routes room_show,messages_page,sidebar,search,post_message
bin/benchmark --apps rails,express,laravel --rounds 3 --duration 8 --concurrencies 16 \
  --cpus 8-11 --client-cpus 12-15 --mixed-write-rate 10 \
  --routes room_show,messages_page,sidebar,search
```

The standalone [verification repository](https://github.com/basecamp/once-campfire-verification) documents per-app image configuration and fresh disposable browser runs. Its lock prevents overlapping benchmark sessions. Run native/framework tests separately, then stop builds/tests/browser workloads before timing. Raw JSON, logs, seeds and runtime databases remain ignored local artifacts; this report commits only a Markdown result summary.

## Contracts and limits

Unsafe browser requests require same-origin or same-site Fetch Metadata, with provided Origin matching the effective URL; null/foreign origins and cross-site/none/invalid metadata are rejected. Missing metadata is allowed only for plain HTTP without forced TLS. Each implementation retains its native trusted-proxy boundary; Rails uses native header-only protection and normalization. Actual bot credentials and signed upload capabilities retain each implementation’s established native authorization exceptions. Installation cookie/schema/storage compatibility and old token-bearing tabs remain intact; new forms/uploads generate no tokens.

The 16 read clients share one freshly authenticated fixture viewer/session per app and round. Warm reuse for one viewer does not measure multi-viewer eviction pressure. Every warmup/timed response must match complete status, MIME, wire length, encoding, UTF-8 and route-content contracts. Expected IDs, order and text come from the fixture independently. Every acknowledged write must match exact database ID, room, rich text and FTS entry, with no duplicates, missing or extra writes. Repeated identical bodies may reuse a completed content check; headers are checked on every response.

Databases/files run on `/tmp` tmpfs. Exact transaction/FTS persistence is verified, but sustained NVMe throughput and crash-safe durability are not measured. Per-process cache payload limits do not bound total RAM. HTTP throughput establishes no concurrent-people count, WebSocket fanout, Internet performance, notification delivery, Raspberry Pi capacity or other-hardware connection ceiling. The optional Cable profile counts supplied sessions; no active-user-capacity or C++ performance claim is made here.

## Independent review

Claude (Opus 5.5) reviewed the earlier changes from all eight public implementations; reproduced gzip-quality and upload-metadata issues were fixed and verified as recorded in the [prior report](https://github.com/basecamp/once-campfire-verification/blob/70e612c1770be352e94d8ccb74184dce74ea6d33/docs/performance-review.md#independent-review). That review predates the additional Rails diff. A supplemental source review covered the specifically supplied `59e1c6d` snapshot. Independent checks against current `2e89e08` confirmed that the checkpoint thread’s sleep/backoff and unbounded shutdown join can delay or stall shutdown/fork preparation; a separate shutdown correction is being prepared and is not included in the measured `7331d3a` source or the supplemental reviewed snapshot. Its final revision and verification evidence remain pending. Other cache/token observations were already addressed or do not apply to current tokenless rendering. The newer Rails runtime/cache/message changes beyond the reviewed 59e1 subset remain outside that supplemental review; native and production checks are separate evidence.

## Contributor credit

Earlier contributor credit is retained: sernle's full-sidebar correction, Nick Potts's renderer incorporated by nijaru, Pasi Vuorio's literal-regex cache, and Jack Ellis's cache identity/token ideas. Daniel Collin ([emoon](https://github.com/emoon)) contributed Rust database scheduling, rich-text rendering and cached-page gzip improvements in [#43](https://github.com/basecamp/once-campfire-rust/pull/43). The C fork retains [mrsaraiva's original history](https://github.com/mrsaraiva/once-campfire-c). Complete earlier acknowledgments and decisions remain in the [pinned historical ledger](https://github.com/basecamp/once-campfire-verification/blob/7b2dbc7e856fb2cbf2d95833a22110beab026eec/docs/performance-review.md#reviewed-proposals).

## Additional contributions and review decisions

Accepted changes retain their contributors’ history. Merge commits use **GPT on behalf of DHH**. The decisions below add no unmeasured performance claims; the benchmark tables report only independently validated runs.

| Contribution | Decision and scope |
|---|---|
| Elmoaid, [Rust #42](https://github.com/basecamp/once-campfire-rust/pull/42) | Merged. Cable replay waits for its expected frames within one deadline, and `dom_id` uses the existing shared helper. Focused replay, channel tests, formatting and Clippy passed. This improves test reliability and removes duplication; it changes no runtime behavior. |
| Marcello Costagliola (@namespaceMarcello), [verification #1](https://github.com/basecamp/once-campfire-verification/pull/1) | Merged with reporting and timer corrections. Adds an optional Cable profile using separate supplied sessions. Reports count sessions, not independently verified people, and describe differing subscription counts accurately. No user-capacity or fan-out throughput figure is certified. |
| @ronakjain90, [verification #2](https://github.com/basecamp/once-campfire-verification/pull/2) | Merged. Adds an optional C++ adapter while retaining the default implementation set and strict response/write validation. This does not certify the C++ implementation or add benchmark figures for it. The combined harness passed its native checks. |
| Marcello Costagliola, [Rails #335](https://github.com/basecamp/once-campfire/pull/335) | Incorporated. Malformed or oversized QR-code requests return 400 instead of a server error. Focused regressions passed; no speedup is attributed to this correction.  [Verified merge](https://github.com/basecamp/once-campfire/pull/335#issuecomment-6059068831). |
| Marcello Costagliola, [Rails #336](https://github.com/basecamp/once-campfire/pull/336) | Incorporated. Avoids rewriting memberships already marked unread, while preserving sidebar and unread behavior.  [Verified merge](https://github.com/basecamp/once-campfire/pull/336#issuecomment-6059069221). |
| Thomas Klemm, [Rails #339](https://github.com/basecamp/once-campfire/pull/339) | Incorporated. Moves SQLite WAL checkpointing off request threads, with one elected checkpoint owner, failover and fork-safe cleanup.  [Verified merge](https://github.com/basecamp/once-campfire/pull/339#issuecomment-6059069689). |
| Sam Ruby, [Rails #303](https://github.com/basecamp/once-campfire/pull/303) | Already incorporated with authorship retained. Makes PWA help and account settings compatible with Herb template compilation. [Review follow-up](https://github.com/basecamp/once-campfire/pull/303#issuecomment-6057243752). |
| Paweł Stachula, [Express #4](https://github.com/basecamp/once-campfire-express/pull/4) | Selected two-file extraction: fewer creation queries and reuse of the exact published fragment. Express and Node/Bun support are retained; co-author credit preserves the contribution without merging the full framework replacement. |
| Silvio Ney, [Laravel #5](https://github.com/basecamp/once-campfire-laravel/pull/5) | The reviewed `0480481` ancestor is incorporated with original contributor history, atomic message/FTS/unread corrections and current SGID mention resolution in measured/published `3b4a889`. Native 80 tests / 1,005 assertions and production controls pass. The author's subsequent `c83d2fa` changes are outside that measured integration; the latest full PR is not claimed merged. |

Thomas Klemm’s [Rails #341](https://github.com/basecamp/once-campfire/pull/341), [#342](https://github.com/basecamp/once-campfire/pull/342), [#343](https://github.com/basecamp/once-campfire/pull/343) and [#344](https://github.com/basecamp/once-campfire/pull/344) are incorporated in the measured Rails source with their original contributor histories. They strengthen libvips loader-block tests, update the RuboCop toolchain, upgrade Puma to 8.0.2 and upgrade Ruby to 4.0.7. Independent integrated native/style/security, 27-case Chromium and exporter checks passed. The updated Ruby PR includes the same already-tested libvips and RuboCop changes: a source-only integration produces an identical runtime tree. The first two changes do not affect application throughput. Posted merge verification: [#341](https://github.com/basecamp/once-campfire/pull/341#issuecomment-6059070132), [#342](https://github.com/basecamp/once-campfire/pull/342#issuecomment-6059070543), [#343](https://github.com/basecamp/once-campfire/pull/343#issuecomment-6059070965), [#344](https://github.com/basecamp/once-campfire/pull/344#issuecomment-6059071349).

Exploratory two-round, five-second comparisons found Puma 8 roughly flat and Ruby 4 about 9% faster for posting and modest read gains. These short comparisons guided candidate selection; they are not the final table or an isolated causal claim. The complete final Rails configuration, including its architectural changes and worker layout, is measured in the tables above. A further two-round native action-controls reuse experiment reached 323.0–333.7 posts/sec (median 328.35), overlapping the final baseline range; its extra identity-substitution machinery was not adopted.

These proposals remain unmerged in full at the listed revisions:

| Contribution and source revision | Status |
|---|---|
| @kidandcat, [Go #5](https://github.com/basecamp/once-campfire-go/pull/5), `a996016` | The updated branch fixes the previously identified generation and pre-lock observation issues. A reproduced remaining race consumes a foreign SQLite commit after a local commit without advancing the content generation, leaving generation-keyed HTML eligible for stale reuse. [Current findings](https://github.com/basecamp/once-campfire-go/pull/5#issuecomment-6057242182). |
| @sebishogun, [Go #10](https://github.com/basecamp/once-campfire-go/pull/10), `c46785c` | Independent controls reproduced private search results and Cable delivery surviving membership revocation by another SQLite connection. Encoding negotiation also selects gzip despite an explicit `gzip;q=0` beyond its first 16 tokens. [Findings](https://github.com/basecamp/once-campfire-go/pull/10#issuecomment-6057241643). |
| Paweł Stachula (@pstachula-dev), [Express #4](https://github.com/basecamp/once-campfire-express/pull/4), `e8426b0` | The author further updated the full Fastify branch’s foreign-writer cache checks. The framework replacement remains unmerged because the selected implementation keeps Express. Creation/query and broadcast-render reuse were extracted with co-author credit; existing global-epoch cache guards remain intact, avoiding the full proposal’s finer-grained revalidation machinery. [Partial adoption and current scope](https://github.com/basecamp/once-campfire-express/pull/4#issuecomment-6060828940). |
| Kurtis Melby (@kurtome), [Elixir #7](https://github.com/basecamp/once-campfire-elixir/pull/7), `332d55c` | Source review confirms generation capture after presentation reads without a final admission check, non-atomic concurrent ETS budget accounting, and unaccounted static entries. The old-base branch also retains timestamp-only nested fragments and restores token APIs removed on current main. Its central whole-response cache idea is already implemented on main with stronger freshness guards. [Findings](https://github.com/basecamp/once-campfire-elixir/pull/7#issuecomment-6057242687). |
| Silvio Ney, [Laravel #5](https://github.com/basecamp/once-campfire-laravel/pull/5), `c83d2fa` | The measured integration includes the original `0480481` proposal and correctness corrections. New upload preparation and lock/backoff creation changes appeared after the frozen benchmark and publication; their review is pending and no new performance or full-PR merge claim is made. |
| Thomas Klemm, [Rails #345](https://github.com/basecamp/once-campfire/pull/345), `47682c2` | Held operational draft. New auxiliary WAL checkpointing resolves the earlier integration gap; no current cache-correctness blocker was identified by source review. Solid Cache leaves hot memory caches and Redis for Cable in place; 256 MiB is an estimated eviction target. Exact-head CI is green; independent production/upgrade/concurrent-rate-limit/restore evidence remains pending, and no speed gain is certified. [Current source review](https://github.com/basecamp/once-campfire/pull/345#issuecomment-6059970433). |
| Thomas Klemm, [Rails #346](https://github.com/basecamp/once-campfire/pull/346), `3914ce5` | Held operational draft. Auxiliary checkpointing, fork hooks, independent pool sizing and quiesced backup guidance have all been corrected in the updated source. Independent production job delivery/failure/retry, upgrade and restore checks remain pending; no HTTP throughput gain is certified. [Updated source review](https://github.com/basecamp/once-campfire/pull/346#issuecomment-6060731980). |

Smaller ideas remain worth extracting with contributor credit: clearly bounded SQLite write batching and Cable recipient indexing from Go #10, transaction retry and deferred-read improvements from Express #4, and bounded immutable asset bytes with precompressed gzip from Express #4 and Elixir #7. Each extraction needs its own correctness checks and measurements; none is counted as a gain here. Architectural transfers remain ongoing. Rails still has a substantial posting-throughput gap despite its improved read and write paths.

Earlier decisions and contributor acknowledgments remain in the [previous review ledger](https://github.com/basecamp/once-campfire-verification/blob/7b2dbc7e856fb2cbf2d95833a22110beab026eec/docs/performance-review.md#reviewed-proposals).
