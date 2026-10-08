# Campfire performance and verification, 2026-10-08

All eight production implementations use Fetch Metadata for browser writes and reuse complete cached HTML/gzip representations where supported. Authentication and room access remain fresh on every hit; SQLite commit epochs prevent stale cache admission.

## Current production HTTP results

Measured with 16 concurrent clients on an AMD Ryzen AI MAX+ 395 with 32 GB RAM,
with four hardware cores allocated to each app.

| HTTP workload (requests/sec) | Rails | [Django](https://github.com/basecamp/once-campfire-django) | [Laravel](https://github.com/basecamp/once-campfire-laravel) | [Express](https://github.com/basecamp/once-campfire-express) | [Elixir](https://github.com/basecamp/once-campfire-elixir) | [Go](https://github.com/basecamp/once-campfire-go) | [Rust](https://github.com/basecamp/once-campfire-rust) | [C](https://github.com/basecamp/once-campfire-c) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 2,063 | 478 | 3,038 | 43,925 | 5,350 | 53,060 | 106,494 | 137,524 |
| Messages page | 2,063 | 486 | 3,081 | 74,176 | 5,712 | 54,800 | 102,697 | 144,642 |
| Sidebar | 2,545 | 601 | 3,832 | 94,322 | 5,949 | 59,144 | 120,294 | 152,002 |
| Search | 2,528 | 594 | 3,710 | 82,937 | 5,848 | 60,509 | 121,378 | 149,487 |
| Post a message | 234 | 112 | 577 | 2,098 | 1,278 | 9,021 | 8,037 | 7,530 |

Medians of 3 alternating rounds, each with a two-second warmup and 8-second sample. Apps run serially on CPUs 8-11; the generator uses 12-15. Builds, tests and other benchmarks were stopped.
39,302,524 timed responses passed, with zero errors or invalid responses; the exact database/FTS audit verified 854,574 acknowledged warmup and timed writes. Peak generator CPU: 95.8% of four-core 400% capacity.

Observed minimum–maximum requests/sec:

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 2,044–2,070 | 2,059–2,065 | 2,535–2,549 | 2,521–2,546 | 227–235 |
| Django | 478–484 | 485–490 | 599–605 | 593–599 | 112–114 |
| Laravel | 3,032–3,097 | 3,071–3,134 | 3,813–3,914 | 3,700–3,764 | 570–587 |
| Express | 43,451–44,037 | 74,138–74,304 | 91,762–95,989 | 82,629–84,724 | 2,090–2,141 |
| Elixir | 5,342–5,360 | 5,686–5,717 | 5,923–5,982 | 5,821–5,937 | 1,278–1,298 |
| Go | 52,795–53,610 | 54,720–54,811 | 58,585–60,178 | 60,088–61,007 | 9,002–9,098 |
| Rust | 105,504–107,910 | 102,687–103,274 | 119,011–121,793 | 120,984–124,036 | 7,988–8,038 |
| C | 136,404–139,097 | 143,942–149,113 | 151,566–152,698 | 148,288–151,008 | 7,512–7,556 |

## Compared with the previous published configuration

The [previous published report](https://github.com/basecamp/once-campfire-verification/blob/7b2dbc7e856fb2cbf2d95833a22110beab026eec/docs/performance-review.md) retains the prior architecture-transfer results, PR decisions and contributor attribution. The tables below compare those complete configurations with the current ones; they do not isolate the cost of a language or a single security/cache change. Regressions are retained.

| Implementation | Room page | Messages page | Sidebar | Search | Post a message |
|---|---:|---:|---:|---:|---:|
| Rails | 710 → 2,063 (2.90×) | 1,113 → 2,063 (1.85×) | 1,901 → 2,545 (1.34×) | 1,332 → 2,528 (1.90×) | 226 → 234 (1.03×) |
| Django | 414 → 478 (1.15×) | 454 → 486 (1.07×) | 576 → 601 (1.04×) | 549 → 594 (1.08×) | 113 → 112 (1.00×) |
| Laravel | 1,696 → 3,038 (1.79×) | 1,890 → 3,081 (1.63×) | 3,364 → 3,832 (1.14×) | 2,615 → 3,710 (1.42×) | 567 → 577 (1.02×) |
| Express | 42,481 → 43,925 (1.03×) | 74,779 → 74,176 (0.99×) | 94,460 → 94,322 (1.00×) | 83,493 → 82,937 (0.99×) | 2,121 → 2,098 (0.99×) |
| Elixir | 1,126 → 5,350 (4.75×) | 1,407 → 5,712 (4.06×) | 3,621 → 5,949 (1.64×) | 2,127 → 5,848 (2.75×) | 1,392 → 1,278 (0.92×) |
| Go | 52,512 → 53,060 (1.01×) | 54,100 → 54,800 (1.01×) | 58,714 → 59,144 (1.01×) | 60,444 → 60,509 (1.00×) | 9,000 → 9,021 (1.00×) |
| Rust | 105,909 → 106,494 (1.01×) | 103,301 → 102,697 (0.99×) | 120,930 → 120,294 (0.99×) | 121,502 → 121,378 (1.00×) | 8,004 → 8,037 (1.00×) |
| C | 137,505 → 137,524 (1.00×) | 142,669 → 144,642 (1.01×) | 151,001 → 152,002 (1.01×) | 148,766 → 149,487 (1.00×) | 7,486 → 7,530 (1.01×) |

Both runs use the same seed, 16-client route/content contracts, gzip, network and CPU allocation. The current generator permits forms without legacy CSRF tokens; it still sends canonical Fetch Metadata, and validates the same complete response content and persisted acknowledgments. Its revision and binary hash differ and are shown below; this is not a claim that the generators are byte-identical.

## Reads with a paced message writer

This separate profile runs 16 read clients and one writer capped at 10 messages/sec,
with no catch-up bursts. Each route has a two-second warmup and
3 rounds of 8-second timed samples. Sources, production images,
fixture, generator and CPU allocations match the regular comparison above.

| Mixed HTTP workload (read requests/sec) | Rails | Django | Laravel | Express | Elixir | Go | Rust | C |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Room page | 219 | 58 | 1,758 | 37,455 | 1,438 | 48,465 | 99,660 | 130,733 |
| Messages page | 221 | 61 | 1,862 | 65,935 | 1,694 | 49,902 | 98,055 | 137,520 |
| Sidebar | 2,046 | 236 | 3,377 | 87,999 | 5,602 | 59,331 | 117,710 | 149,740 |
| Search | 1,009 | 107 | 2,973 | 75,347 | 4,009 | 58,944 | 117,919 | 145,166 |

Achieved writer cadence across the timed route samples:

| Implementation | Timed acknowledged writes | Writes per sample (min–max) | Writes/sec (min–max) |
|---|---:|---:|---:|
| Rails | 803 | 47–80 | 5.7–10.0 |
| Django | 830 | 65–79 | 8.0–9.9 |
| Laravel | 960 | 80–80 | 9.9–10.0 |
| Express | 960 | 80–80 | 9.9–9.9 |
| Elixir | 960 | 80–80 | 9.9–10.0 |
| Go | 960 | 80–80 | 9.9–9.9 |
| Rust | 960 | 80–80 | 9.9–9.9 |
| C | 960 | 80–80 | 9.9–9.9 |

**36,070,827 timed reads and 7,393 timed writes**
passed their response contracts, totaling **36,078,220 validated timed responses**
with zero request errors or invalid responses. The exact database/FTS audit verified
all **9,282 acknowledged writes**, including
1,889 warmup writes. Raw warmup and timed receipts
were reconciled with every application/round audit. The writer cap is a requested maximum,
rather than an assumption that every application achieved it.
Peak combined reader/writer generator CPU was 92.9% of its four-core 400% capacity.


The paced writer posts to the HQ fixture room; room/message reads target Watercooler. Current global SQLite epochs invalidate those pages even for writes to another room. Sidebar/search exercise their normal scopes. This profile measures cross-room invalidation and concurrent writes; it is not a measurement of fanout or every reader following one actively written room.

Observed mixed read minimum–maximum requests/sec:

| Implementation | Room page | Messages page | Sidebar | Search |
|---|---:|---:|---:|---:|
| Rails | 217–291 | 174–230 | 1,993–2,060 | 1,009–1,026 |
| Django | 55–58 | 59–61 | 216–237 | 105–108 |
| Laravel | 1,754–1,781 | 1,855–1,879 | 3,247–3,407 | 2,960–3,011 |
| Express | 37,445–38,628 | 65,703–67,271 | 70,969–88,567 | 75,174–76,265 |
| Elixir | 1,378–1,526 | 1,650–1,713 | 5,554–5,610 | 3,930–4,031 |
| Go | 48,054–48,680 | 49,793–50,521 | 58,774–59,661 | 58,688–59,050 |
| Rust | 99,370–100,957 | 98,012–98,126 | 117,399–119,116 | 116,797–118,859 |
| C | 130,011–130,879 | 137,449–137,649 | 148,506–151,328 | 145,105–145,584 |

Mixed-profile earlier → current median requests/sec:

| Implementation | Room page | Messages page | Sidebar | Search |
|---|---:|---:|---:|---:|
| Rails | 70 → 219 (3.13×) | 84 → 221 (2.63×) | 1,507 → 2,046 (1.36×) | 517 → 1,009 (1.95×) |
| Django | 55 → 58 (1.05×) | 61 → 61 (1.00×) | 228 → 236 (1.03×) | 104 → 107 (1.03×) |
| Laravel | 1,059 → 1,758 (1.66×) | 1,220 → 1,862 (1.53×) | 2,916 → 3,377 (1.16×) | 2,185 → 2,973 (1.36×) |
| Express | 36,823 → 37,455 (1.02×) | 66,955 → 65,935 (0.98×) | 87,056 → 87,999 (1.01×) | 76,414 → 75,347 (0.99×) |
| Elixir | 264 → 1,438 (5.46×) | 350 → 1,694 (4.83×) | 3,476 → 5,602 (1.61×) | 1,317 → 4,009 (3.04×) |
| Go | 48,272 → 48,465 (1.00×) | 50,488 → 49,902 (0.99×) | 59,301 → 59,331 (1.00×) | 59,163 → 58,944 (1.00×) |
| Rust | 101,129 → 99,660 (0.99×) | 98,303 → 98,055 (1.00×) | 119,850 → 117,710 (0.98×) | 117,670 → 117,919 (1.00×) |
| C | 128,459 → 130,733 (1.02×) | 137,625 → 137,520 (1.00×) | 149,652 → 149,740 (1.00×) | 146,149 → 145,166 (0.99×) |

| Implementation | Earlier writer requests/sec (min–max) | Current writer requests/sec (min–max) |
|---|---:|---:|
| Rails | 6.2–10.0 | 5.7–10.0 |
| Django | 7.7–10.0 | 8.0–9.9 |
| Laravel | 9.9–10.0 | 9.9–10.0 |
| Express | 9.9–9.9 | 9.9–9.9 |
| Elixir | 9.9–10.0 | 9.9–10.0 |
| Go | 9.9–9.9 | 9.9–9.9 |
| Rust | 9.9–9.9 | 9.9–9.9 |
| C | 9.9–9.9 | 9.9–9.9 |

The writer cap and room match the prior mixed profile. Achieved writer cadence is reported above; cache churn and different achieved cadence prevent treating this as an isolated cache benchmark.

## Source, images and verification

| Implementation | Measured source | Frozen image | Native checks |
|---|---|---|---|
| Rails | [`0f5d0b2`](https://github.com/basecamp/once-campfire/commit/0f5d0b2b6e3b79fe47ee7b32dbccb75d6ea663a7) | `sha256:19033001ec30d33d4570ea1d84d320ebc3a8cf5646a29890b0c601a57ceee46e` | 539 / 2030 assertions; system 27 / 203 |
| Django | [`04f1f27`](https://github.com/basecamp/once-campfire-django/commit/04f1f273f43ab3f0c66565082ca3fd31eb994ce6) | `sha256:1d1a7d4226d7881d418054dbde2c4ad258ac604faee01763eb91846af506c843` | 64 |
| Laravel | [`a0a35fa`](https://github.com/basecamp/once-campfire-laravel/commit/a0a35fa5fdead6d73bfb333e3b3d634ce7b8776e) | `sha256:4d065b1cb5441dd725ea103a31937a2ae796e37851fb94e1a7b1dfa63c93af55` | 76 / 975 assertions |
| Express | [`f63ecd6`](https://github.com/basecamp/once-campfire-express/commit/f63ecd6edb0044cde07bf36ca2574510fbbcd73a) | `sha256:4fda26e64ea0a262d148e440de0aed63e8aa7aabc0573e55dfd4013288b3114d` | 152 |
| Elixir | [`fa7f2ec`](https://github.com/basecamp/once-campfire-elixir/commit/fa7f2ecf89d560bb64a1c215e5693fa285c02705) | `sha256:947432c2355e89bba7dd660e3a5a12fc170d16d5c616d44674a49d8a8943225b` | 1961 |
| Go | [`7fc7412`](https://github.com/basecamp/once-campfire-go/commit/7fc7412db06e1f3520d4377d5fdc3855f65abfb1) | `sha256:03dd26fb3aff9ed445a8c28833aa0eb7c060ae2ac1c29c9d4ba089e0dbb1dc06` | Full race suite and vet |
| Rust | [`f9dca47`](https://github.com/basecamp/once-campfire-rust/commit/f9dca47e592fdcd23a75e4c8b6b97f5d452ccdc1) | `sha256:a368be61a026aa8f78d04019beb8bfb2870db53bf299ac2fa7cd116b6268d87f` | 795 passed; 11 existing ignores |
| C | [`cd0cbe4`](https://github.com/basecamp/once-campfire-c/commit/cd0cbe4b24bcee0aa6ad75e6bc29d4e773628332) | `sha256:41da1ded88b26948a85780afc7ec8dc4bcc6082f8b7c70e8dfd26f9c94d2d404` | 1676 full historical native cases; current Fetch Metadata affected subset is separately recorded |

Current harness: [`b95d5a7`](https://github.com/basecamp/once-campfire-verification/commit/b95d5a7bfbc8ce1ae03d01d293d19dd719f0d86d).
Current seed SHA-256: `036edd6a07815bbddbfe60949d065caf777cee6f0c1a96e286e08c16e584c60c`.
Earlier generator SHA-256: `3decb2029d24156eba180ebfe9126f10d8dfce650743944660018c7fe82783b0`; current: `26ac5ed0a67b04867583c3745da4c266b417750fb2309e47bc3357c206735025`.

Fresh strict Chromium production gates cover setup, live messaging, editing, stored-markup safety, profile/account changes, bots, styles, transfers, joining and direct pings. Separate native/HTTP gates cover signed-upload capabilities. External SQL edits without timestamp changes, stale conditionals and revoked sessions/memberships are checked against the final images. Runtime-equivalent documentation/test-only commits above a frozen image are identified in the local freeze receipt; image and source identifiers are not silently substituted.

## Browser-write protection and cache limits

GET and HEAD remain available. Unsafe browser requests require same-origin or same-site Fetch Metadata, with provided Origin matching the effective URL; null/foreign origins and cross-site/none/invalid metadata are rejected. Missing metadata is allowed only for plain HTTP without forced TLS. Each implementation retains its established trusted-proxy boundary. Rails uses its installed native header-only strategy and its native value normalization. Actual bot credentials and signed upload capabilities retain their existing native exceptions. Installation cookies and old token-bearing tabs remain compatible; new forms and upload headers emit no CSRF tokens.

Completed representations contain no per-request CSRF mask. Cached headers exclude live cookies; authentication and authorization run on every lookup. Admission rechecks the captured SQLite epoch, and nested fragment namespaces retain that epoch. These are per-process bounded payload caches, not a bound on total application RAM.

The 16 clients share one freshly authenticated fixture user/session per app and round. Repeated reads measure warm reuse for one viewer; distinct-viewer eviction pressure is not measured. Every timed and warmup response must pass complete status/MIME/wire-length/encoding/UTF-8 and route-content contracts. Expected IDs, order and text come from the fixture independently. Every acknowledged message is checked by exact ID, room, stored rich text and FTS entry; duplicate/missing/extra writes fail the run. Repeated identical bodies may reuse a completed content check, while headers are checked on every response.

Disposable databases and files live on /tmp tmpfs. Exact transaction/FTS persistence is audited; sustained NVMe throughput and crash-safe disk durability are not measured. HTTP throughput does not establish active-chat capacity, WebSocket fanout, Internet performance, durable notification delivery or connection capacity on other hardware. The separate paced-writer profile exposes invalidation cost, not sustained real-user chat capacity.

Rails has two existing unsupported-libvips-loader skips; its 27 system tests have no skips. Rust has 11 existing ignored timing/reference-export/doc cases and no missing-seed skips. Django and Elixir current suites have no skips. C runtime is unchanged from its previously verified full 1,676-case suite; 126 affected native cases were rerun. Framework formatting/static checks passed. The prior full Elixir parity inventory, Rust multi-seed inventories and complete C sanitizer/Fil-C receipts predate this change; fresh shared production gates supplement them, and this report does not claim all historical parity inventories were rerun. Each port retains its documented rich-text/media/push/upgrade limits.

## Independent review

Claude (Opus 5.5) independently reviewed the prepared changes from all eight public implementations. Confirmed findings were reproduced and fixed: Django gzip selection now respects quality exclusions; Elixir direct-upload metadata validates object types and old stored metadata safely; optional forced-TLS flags accept true/1 consistently. Native regressions and exact production gates cover those changes. The final follow-up review disposition is recorded below.
The follow-up review found no blocking regression. Remaining questions were checked against the full source: cached encodings have separate keys, Django always binds the actual upload owner, and Elixir has no other forced-TLS flag reader.

## Contributor credit

Contributor commits and partial-work co-author trailers preserve credit for sernle's full-sidebar
correction, Nick Potts's renderer incorporated by nijaru, Pasi Vuorio's literal-regex cache and Jack
Ellis's cache identity/token ideas. Daniel Collin ([emoon](https://github.com/emoon)) contributed
Rust database scheduling, rich-text rendering and cached-page gzip improvements in
[#43](https://github.com/basecamp/once-campfire-rust/pull/43). The public C fork retains
[mrsaraiva's original history](https://github.com/mrsaraiva/once-campfire-c).

Original contributor commits and credits remain intact. The [pinned previous report](https://github.com/basecamp/once-campfire-verification/blob/7b2dbc7e856fb2cbf2d95833a22110beab026eec/docs/performance-review.md#revisions-and-attribution) preserves the complete acknowledgments and [PR decision ledger](https://github.com/basecamp/once-campfire-verification/blob/7b2dbc7e856fb2cbf2d95833a22110beab026eec/docs/performance-review.md#reviewed-proposals).
