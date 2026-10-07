# Architecture transfer inventory

Source inspection of the eight implementations on October 7, 2026. This records
implemented mechanisms and remaining opportunities, not measured gains or complete
behavioral parity. The optional mixed read/write profile measures cache churn separately
from the normal comparison. New performance claims require production validation and
sequential measurements of the final source and images.

| Implementation | Scoped bounded FTS probe and exact direct-room lookup | Authenticated versioned page cache | Cached representation | Background WAL checkpoint | Notification queue |
|---|---|---|---|---|---|
| Rails | SQL probe/fallback; grouped membership query | Read-only SQLite observer; fresh authentication/access and CSRF masks | HTML; compression remains in the front server | SQLite autocheckpoint | Redis/Resque |
| Django | SQL probe/fallback; ORM grouped membership query | Read-only SQLite observer; fresh authentication/access and CSRF masks | HTML; normal middleware compression | SQLite autocheckpoint | Leased auxiliary SQLite |
| Laravel | SQL probe/fallback; grouped membership query | Read-only SQLite observer; fresh authentication/access and session CSRF | HTML; FrankenPHP compression | SQLite autocheckpoint | Auxiliary SQLite |
| Express | SQL probe/fallback; grouped membership query | Local write epoch plus persistent connection's foreign-commit version; fresh authentication/access | Completed HTML and gzip; fragment deflate reuse | Dedicated checkpoint worker and writer fallback | Leased auxiliary SQLite, batched admission |
| Elixir | SQL probe/fallback; grouped membership query | Dedicated database observer; fresh authentication/access and CSRF masks | HTML; normal Plug compression | SQLite autocheckpoint | Redis/Resque |
| Go | SQL probe/fallback; grouped membership query | Pinned read-only SQLite observer; fresh authentication/access | Completed HTML/gzip; immutable fragment/list/layout parts | SQLite autocheckpoint | Bounded in-process |
| Rust | SQL probe/fallback; grouped membership query | Dedicated observer; fresh authentication/access | Completed bodies plus fragment deflate reuse | Dedicated checkpointer with writer fallback | Bounded in-process |
| C | SQL probe/fallback; grouped membership query | Dedicated version source; authorization checked on each lookup | Completed bodies and gzip | SQLite autocheckpoint | Bounded in-process |

Compiled ERB, Jinja, Blade, Eta, EEx, Go templates, Askama and C renderers are equivalent
ways to avoid repeatedly parsing templates. A framework does not need replacing to obtain
that benefit. Each implementation has production assets and native database access;
prepared-statement reuse is explicit in Go, Rust, Express and Elixir. Other frameworks
use their own driver connections; this does not establish reuse across requests.
Pooling and session behavior differ, so a larger
worker count is not automatically an improvement. Multiple HTTP workers require shared
broadcast delivery; an isolated process's subscriber list cannot deliver to another worker.

A page-cache miss must not recover stale HTML from a nested fragment cache. Go and Express
namespace message fragments by the observed database generation, including foreign SQL
edits that do not update timestamps. Requests captured before a commit cannot populate the
newer namespace. The other implementations' admission and fragment rules need their own
external-write regressions; a whole-page generation check alone proves neither property.

Worth considering next:

- Background WAL checkpoints for the implementations still using SQLite's foreground
  autocheckpoint, with a writer fallback that bounds WAL growth. Compare write latency and
  resource use under the mixed profile before changing defaults.
- Persistent, serializable jobs for Go, Rust and C if crash-safe delivery is required.
  Graceful draining of an in-process queue is not durability. This is a product behavior
  change and requires retry, lease and idempotency contracts, not merely a faster queue.
- Completed compressed-body reuse where CSRF masks currently require fresh identity HTML.
  Preserve the framework's token policy; compressing an old mask or replacing arbitrary
  message text is not a valid optimization. Measure whether compression is actually material.
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
