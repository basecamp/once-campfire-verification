# Architecture transfer inventory

Source inspection of the eight implementations on October 8, 2026. This records
implemented mechanisms and remaining opportunities, not measured gains or complete
behavioral parity. The optional mixed read/write profile measures cache churn separately
from the normal comparison. New performance claims require production validation and
sequential measurements of the final source and images.

| Implementation | Scoped bounded FTS probe and exact direct-room lookup | Authenticated versioned page cache | Cached representation | Background WAL checkpoint | Notification queue |
|---|---|---|---|---|---|
| Rails | SQL probe/fallback; grouped membership query | Read-only SQLite observer; fresh authentication/access | Completed HTML/gzip; immutable encoded bodies | SQLite autocheckpoint | Redis/Resque |
| Django | SQL probe/fallback; ORM grouped membership query | Read-only SQLite observer; fresh authentication/access | Completed HTML/gzip; normal middleware fallback | SQLite autocheckpoint | Leased auxiliary SQLite |
| Laravel | SQL probe/fallback; grouped membership query | Read-only SQLite observer; fresh authentication/access | Completed HTML/gzip; native server fallback | SQLite autocheckpoint | Auxiliary SQLite |
| Express | SQL probe/fallback; grouped membership query | Local write epoch plus persistent connection's foreign-commit version; fresh authentication/access | Completed HTML and gzip; fragment deflate reuse | Dedicated checkpoint worker and writer fallback | Leased auxiliary SQLite, batched admission |
| Elixir | SQL probe/fallback; grouped membership query | Dedicated database observer; fresh authentication/access | Completed HTML/gzip; normal Plug fallback | SQLite autocheckpoint | Redis/Resque |
| Go | SQL probe/fallback; grouped membership query | Pinned read-only SQLite observer; fresh authentication/access | Completed HTML/gzip; immutable fragment/list/layout parts | SQLite autocheckpoint | Bounded in-process |
| Rust | SQL probe/fallback; grouped membership query | Dedicated observer; fresh authentication/access | Completed bodies plus fragment deflate reuse | Dedicated checkpointer with writer fallback | Bounded in-process |
| C | SQL probe/fallback; grouped membership query | Dedicated version source; fresh authentication/access on each lookup | Completed bodies and gzip | SQLite autocheckpoint | Bounded in-process |

Compiled ERB, Jinja, Blade, Eta, EEx, Go templates, Askama and C renderers are equivalent
ways to avoid repeatedly parsing templates. A framework does not need replacing to obtain
that benefit. Each implementation has production assets and native database access;
prepared-statement reuse is explicit in Go, Rust, Express and Elixir. Other frameworks
use their own driver connections; this does not establish reuse across requests.
Pooling and session behavior differ, so a larger
worker count is not automatically an improvement. Multiple HTTP workers require shared
broadcast delivery; an isolated process's subscriber list cannot deliver to another worker.

Every authenticated page cache captures its SQLite epoch before loading the current user,
checks it at lookup and checks it again before admitting the completed response. A commit
during authentication or rendering therefore cannot publish an old snapshot under the new
epoch. Dedicated read-only observers see both local and foreign commits; Express combines
its persistent connection's foreign-commit version with its local write epoch. SQLite rollbacks
do not invalidate a committed snapshot. Observer failures bypass the optional cache.

A page-cache miss must also avoid stale nested HTML after foreign SQL edits that leave record
timestamps unchanged. Rails retains native ERB collection and Jbuilder caching in a separate
64 MiB memory store per worker, keyed by the pre-authentication epoch, origin, mount path,
format, locale and viewer/session context. All request methods capture the epoch;
detached broadcasts, whose
renderers do not run authentication callbacks, render without fragment caching. Shared
Rails.cache and class-level rate-limit stores stay unchanged. Sixteen fixed render locks
collapse concurrent cold page misses without an unbounded per-key lock map. Laravel
captures the epoch on every route and shares one 64 MiB budget between completed pages and
native message/boost fragments; detached renders without a captured snapshot
bypass reuse. Elixir
also namespaces native fallback fragments by the pre-render epoch and request host, including
conditional requests and when whole-response caching is disabled; its existing store has a
4,096-entry bound. Django renders its page body directly. Go, Rust and Express namespace nested fragments by the
request's observed database generation. Old in-flight renders retain their old namespace,
while all namespaces share the existing byte budget. Go and Rust also separate fragments by
request origin; nested OpenGraph presentation and absolute links can depend on that origin.
Rust also captures detached presentation snapshots before database reads and isolates their
absence of an HTTP origin from request renders. Native regressions target
foreign writes and stale admissions; a whole-page version check alone would not prove these
nested-cache properties.

Rails, Django, Laravel, Express, Elixir, Go and Rust use `CAMPFIRE_RESPONSE_CACHE_MB`;
C uses `CF_CACHE_BYTES` in bytes. The default is 64 MiB (`67,108,864` bytes in C), and zero
disables response reuse. Budgets apply to each process or persistent worker, except
C's shared process store. They account for cached payloads and entry overhead, rather than
limiting total application memory. Nested fragment stores have their own existing bounds.
Live session/cookie middleware remains outside reusable bodies. All eight implementations now
protect unsafe browser writes with Fetch Metadata rather than generated CSRF tokens. Forms emit
no token fields or meta tags, and the upload controller no longer reads a removed token tag.
Complete HTML and selected gzip representations therefore remain reusable without mask hydration
or replacement of arbitrary message content. Cache hits still authenticate and authorize against
current records; conditional requests and flash retain native response rules.

GET and HEAD skip forgery protection. Unsafe requests accept browser-generated `same-origin`
and `same-site`; a provided Origin must match the effective URL, and null/foreign origins and
cross-site/none/invalid metadata fail. Missing metadata is accepted only over plain HTTP without
forced TLS. Each implementation retains its established trusted TLS/proxy boundary; Rails uses
its installed header-only strategy and native header normalization. Real bot credentials and
signed disk upload capabilities preserve existing native exceptions. Old signed/encrypted
installation cookies and token-bearing tabs remain valid. HTTPS clients need Fetch Metadata;
this changes the supported browser policy, rather than weakening response validation.

Message pagination validators must describe the rendered response. Rails, Go, Rust and Elixir
now derive pagination ETags from its actual content and omit timestamp-only Last-Modified
headers. A compatible foreign SQLite writer can change rich text, names or boosts without
touching the message timestamp, so a record-only validator can wrongly return 304. With no
generated token mask in the representation, unchanged visible content does not need a new
validator merely because another request rendered it.

Worth considering next:

- Background WAL checkpoints for the implementations still using SQLite's foreground
  autocheckpoint, with a writer fallback that bounds WAL growth. Compare write latency and
  resource use under the mixed profile before changing defaults.
- Persistent, serializable jobs for Go, Rust and C if crash-safe delivery is required.
  Graceful draining of an in-process queue is not durability. This is a product behavior
  change and requires retry, lease and idempotency contracts, not merely a faster queue.
- Shared immutable broadcast frames where each subscriber still performs equivalent JSON
  encoding/compression. Preserve recipient authorization and bounded slow-client handling;
  Redis/IPC fanout and in-process fanout need different native implementations.

Representative source entry points:

- [Rails response cache](https://github.com/basecamp/once-campfire/blob/main/app/controllers/concerns/cached_responses.rb) and [search](https://github.com/basecamp/once-campfire/blob/main/app/models/message/searchable.rb).
- [Django response cache](https://github.com/basecamp/once-campfire-django/blob/main/campfire/response_cache.py) and [domain queries](https://github.com/basecamp/once-campfire-django/blob/main/campfire/domain.py).
- [Laravel response cache](https://github.com/basecamp/once-campfire-laravel/blob/main/app/Http/Middleware/CacheResponses.php) and [message queries](https://github.com/basecamp/once-campfire-laravel/blob/main/app/Models/Message.php).
- [Express response cache](https://github.com/basecamp/once-campfire-express/blob/main/src/response_cache.js) and [checkpoint worker](https://github.com/basecamp/once-campfire-express/blob/main/src/checkpoint.js).
- [Elixir response cache](https://github.com/basecamp/once-campfire-elixir/blob/main/lib/campfire/response_cache.ex) and [search](https://github.com/basecamp/once-campfire-elixir/blob/main/lib/campfire/searches.ex).
- [Go response cache](https://github.com/basecamp/once-campfire-go/blob/main/internal/web/response_cache.go) and [database](https://github.com/basecamp/once-campfire-go/blob/main/internal/database/database.go).
- [Rust response cache](https://github.com/basecamp/once-campfire-rust/blob/main/crates/campfire/src/response_cache.rs) and [checkpointer](https://github.com/basecamp/once-campfire-rust/blob/main/crates/db/src/database.rs).
- [C cache](https://github.com/basecamp/once-campfire-c/blob/main/src/cache.h) and [database readers](https://github.com/basecamp/once-campfire-c/blob/main/src/db/reader.c).
