//! `loadgen reconnect`: what a server restart costs the people connected to it.
//!
//!   loadgen reconnect --base URL --room ID --sessions FILE --clients N
//!                     [--policy actioncable|herd|early --retry-ms 250 --refresh 1 --connect-timeout-ms 2000]
//!                     [--early-min-ms 1000 --early-max-ms 2000 --early-backoff 1.0]
//!                     [--max-secs 180 --settle-secs 3 --probe-ms 200 --trace FILE]
//!
//! N people (`--sessions`, as for `cable`) hold a room page's cable connection. Once they are all subscribed the
//! command prints `PHASE ready <unix ms>` on stderr: the caller restarts the server then. Every socket drops, and each
//! client gets back in the way `--policy` says:
//!
//! - `actioncable` (the default) replays the browser: `ConnectionMonitor` of `@rails/actioncable`. It doesn't retry
//!   when the socket closes but at the next turn of its poll timer (every 6-12 s at random), never sooner than 6 s
//!   after the close, then every 6 * 1.15^attempts s (+0-15%). A socket that opened but has been silent for 6 s at a
//!   poll is closed and reopened 500 ms later; one still connecting is left alone. Each client's poll timer starts
//!   where one that has been running for hours would be, so the clients are not in step for having connected together.
//! - `herd` retries every `--retry-ms` from the close on, with no wait: the thundering herd the monitor exists to
//!   prevent, which shows what the freshly started server can take.
//! - `early` is the browser's monitor plus a retry of the page's own: a random `--early-min-ms`..`--early-max-ms`
//!   after the close, and again after each failure, the wait multiplied by `--early-backoff` every time.
//!
//! Back in, a client subscribes again and, when HeartbeatChannel is confirmed, asks for the room's news as
//! `refresh_room_controller.js` does (`GET /rooms/ID/refresh?since=..&reason=connection`, `--refresh 0` to skip).
//! A person's blackout runs from their socket closing to all subscriptions confirmed and that response received.
//! `/up` is probed every `--probe-ms` to tell when the server itself went away and came back. A TCP connect that
//! doesn't complete in `--connect-timeout-ms` counts as a failed attempt; the upgrade is waited for without a limit,
//! as a browser does. `--trace FILE` writes one line per client, milliseconds from the first close.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Message as WsMessage, client::IntoClientRequest};

use super::{Args, Res, host_port, one_shot, origin_for, parse_sessions, phase, user_agent};

/// `ConnectionMonitor.staleThreshold`, seconds.
const STALE: f64 = 6.0;
/// `ConnectionMonitor.reconnectionBackoffRate`.
const BACKOFF_RATE: f64 = 0.15;
/// `Connection.reopenDelay`.
const REOPEN_DELAY: Duration = Duration::from_millis(500);

type Ws = WebSocketStream<TcpStream>;

#[derive(Clone, Copy, PartialEq)]
enum Policy {
    ActionCable,
    Herd,
    Early,
}

struct Ctx {
    addr: String,
    refresh_path: Option<String>,
    policy: Policy,
    retry: Duration,
    early: (f64, f64, f64),
    connect_timeout: Duration,
    /// Set at `PHASE ready`: closes before it are the harness failing, not the restart.
    armed: AtomicBool,
    ready: AtomicUsize,
    failed: AtomicUsize,
    back: AtomicUsize,
    /// When every attempt started, for the arrivals per second.
    attempts: Mutex<Vec<Instant>>,
}

/// One client's timeline.
#[derive(Clone, Default)]
struct Record {
    ready: bool,
    early_close: bool,
    close: Option<Instant>,
    first_attempt: Option<Instant>,
    tries: u32,
    failed_tries: u32,
    reopens: u32,
    /// Start of the attempt that got in.
    attempt: Option<Instant>,
    opened: Option<Instant>,
    welcome: Option<Instant>,
    confirmed: Option<Instant>,
    refreshed: Option<Instant>,
    refresh_ms: f64,
    refresh_status: u16,
    back: Option<Instant>,
}

/// xorshift64: the jitter needs no more, and the crate has no `rand`.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `ConnectionMonitor#getPollInterval`.
fn poll_interval(attempts: u32, rng: &mut Rng) -> Duration {
    let backoff = (1.0 + BACKOFF_RATE).powi(attempts.min(10) as i32);
    let jitter_max = if attempts == 0 { 1.0 } else { BACKOFF_RATE };
    Duration::from_secs_f64(STALE * backoff * (1.0 + jitter_max * rng.unit()))
}

/// Where in its cycle a monitor that has been polling for hours is: the time to its next poll. A person's page has
/// been open for long when the server restarts; clients that all connected a few seconds before would poll in step.
fn first_poll(rng: &mut Rng) -> Duration {
    let mut t = -(200.0 + 12.0 * rng.unit());
    while t <= 0.0 {
        t += poll_interval(0, rng).as_secs_f64();
    }
    Duration::from_secs_f64(t)
}

/// Opens the socket: TCP within the timeout, then the upgrade for as long as it takes.
async fn open(addr: String, cookie: String, connect_timeout: Duration) -> Res<Ws> {
    let mut req = format!("ws://{addr}/cable").into_client_request()?;
    let h = req.headers_mut();
    h.insert("cookie", cookie.parse()?);
    h.insert("origin", origin_for(&addr).parse()?);
    h.insert("sec-websocket-protocol", "actioncable-v1-json, actioncable-unsupported".parse()?);
    if let Some(user_agent) = user_agent() {
        h.insert("user-agent", user_agent.parse()?);
    }
    let stream = tokio::time::timeout(connect_timeout, async {
        let target = tokio::net::lookup_host(&addr).await?.next().ok_or("no address")?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(TcpStream::connect(target).await?)
    })
    .await
    .map_err(|_| "connect timed out")??;
    stream.set_nodelay(true)?;
    let (ws, _) = tokio_tungstenite::client_async(req, stream).await?;
    Ok(ws)
}

async fn subscribe(ws: &mut Ws, subs: &[String]) -> Res<()> {
    for ident in subs {
        ws.send(WsMessage::text(json!({"command": "subscribe", "identifier": ident}).to_string())).await?;
    }
    Ok(())
}

async fn next_frame(live: &mut Option<Ws>) -> Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>> {
    match live {
        Some(ws) => ws.next().await,
        None => std::future::pending().await,
    }
}

async fn joined<T>(handle: &mut Option<JoinHandle<T>>) -> Option<T> {
    match handle {
        Some(h) => h.await.ok(),
        None => std::future::pending().await,
    }
}

async fn at(when: Option<Instant>) {
    match when {
        Some(t) => tokio::time::sleep_until(t).await,
        None => std::future::pending().await,
    }
}

/// The room's news since `since_ms`, as the page asks for when its connection comes back.
async fn refresh(addr: String, path: String, cookie: String) -> (f64, u16) {
    let start = Instant::now();
    let headers = [("cookie", cookie), ("accept", "text/vnd.turbo-stream.html, text/html, application/xhtml+xml".to_string())];
    let status = match one_shot(&addr, "GET", &path, &headers, Bytes::new()).await {
        Ok(resp) => resp.status,
        Err(_) => 0,
    };
    (start.elapsed().as_secs_f64() * 1000.0, status)
}

#[allow(clippy::too_many_arguments)]
async fn person(
    ctx: Arc<Ctx>,
    n: usize,
    cookie: String,
    subs: Vec<String>,
    gate: Arc<tokio::sync::Semaphore>,
    mut stop: tokio::sync::watch::Receiver<bool>,
) -> Record {
    let mut rec = Record::default();
    let seed = (n as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ std::process::id() as u64;
    let mut rng = Rng(seed | 1);

    // Before the restart: connect and subscribe, at most 50 handshakes in flight, as `cable` does.
    let mut live = {
        let _permit = gate.acquire().await;
        let Ok(mut ws) = open(ctx.addr.clone(), cookie.clone(), Duration::from_secs(30)).await else {
            ctx.failed.fetch_add(1, Ordering::Relaxed);
            return rec;
        };
        let mut confirms = 0;
        let subscribed = async {
            subscribe(&mut ws, &subs).await.ok()?;
            while confirms < subs.len() {
                if let WsMessage::Text(text) = ws.next().await?.ok()?
                    && text.contains("confirm_subscription")
                {
                    confirms += 1;
                }
            }
            Some(())
        };
        if tokio::time::timeout(Duration::from_secs(60), subscribed).await.ok().flatten().is_none() {
            ctx.failed.fetch_add(1, Ordering::Relaxed);
            return rec;
        }
        Some(ws)
    };
    rec.ready = true;
    ctx.ready.fetch_add(1, Ordering::Relaxed);

    // The monitor's state, named as in connection_monitor.js.
    let mut pinged_at = Instant::now();
    let mut reconnect_attempts = 0u32;
    let mut disconnected_at: Option<Instant> = None;
    let mut disconnected = false;
    let mut next_poll = Instant::now() + first_poll(&mut rng);

    let mut pending: Option<JoinHandle<Res<Ws>>> = None;
    let mut pending_since = Instant::now();
    let mut reopen_at: Option<Instant> = None;
    let mut early_failures = 0i32;
    let early_wait = |failures: i32, rng: &mut Rng| {
        let (min, max, backoff) = ctx.early;
        Duration::from_secs_f64(((min + (max - min) * rng.unit()) * backoff.powi(failures)).min(30.0))
    };
    let mut refreshing: Option<JoinHandle<(f64, u16)>> = None;
    let mut confirms = subs.len();

    loop {
        // What happened; acted on below, once the futures have let go of the state.
        enum Event {
            Poll,
            Reopen,
            Opened(Option<Res<Ws>>),
            Frame(Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>),
            Refreshed(Option<(f64, u16)>),
            Stop,
        }
        let event = tokio::select! {
            _ = tokio::time::sleep_until(next_poll) => Event::Poll,
            _ = at(reopen_at) => Event::Reopen,
            r = joined(&mut pending) => Event::Opened(r),
            f = next_frame(&mut live) => Event::Frame(f),
            r = joined(&mut refreshing) => Event::Refreshed(r),
            _ = stop.changed() => Event::Stop,
        };
        let now = Instant::now();
        let mut start_attempt = false;
        let mut closed = false;
        match event {
            Event::Stop => break,
            Event::Poll => {
                // reconnectIfStale
                if ctx.policy != Policy::Herd && now.duration_since(pinged_at).as_secs_f64() > STALE {
                    reconnect_attempts += 1;
                    let recently = disconnected_at.is_some_and(|d| now.duration_since(d).as_secs_f64() < STALE);
                    if !recently {
                        // Connection#reopen: an open socket is closed and reopened in 500 ms; a connecting one is left alone.
                        if live.is_some() {
                            live = None;
                            closed = true;
                            rec.reopens += 1;
                            reopen_at = Some(now + REOPEN_DELAY);
                        } else if pending.is_none() {
                            start_attempt = true;
                        }
                    }
                }
                next_poll = now + poll_interval(reconnect_attempts, &mut rng);
            }
            Event::Reopen => {
                reopen_at = None;
                start_attempt = live.is_none() && pending.is_none();
            }
            Event::Opened(result) => {
                pending = None;
                match result {
                    Some(Ok(ws)) => {
                        live = Some(ws);
                        disconnected = false;
                        early_failures = 0;
                        confirms = 0;
                        rec.attempt = Some(pending_since);
                        rec.opened = Some(now);
                    }
                    _ => {
                        rec.failed_tries += 1;
                        match ctx.policy {
                            Policy::Herd => reopen_at = Some(now + ctx.retry),
                            Policy::Early => {
                                early_failures += 1;
                                reopen_at = Some(now + early_wait(early_failures, &mut rng));
                            }
                            Policy::ActionCable => {}
                        }
                    }
                }
            }
            Event::Frame(frame) => match frame {
                Some(Ok(WsMessage::Text(text))) => {
                    pinged_at = now; // recordMessage: every message counts, not only pings
                    if text.contains(r#""type":"welcome""#) {
                        reconnect_attempts = 0; // recordConnect
                        disconnected_at = None;
                        rec.welcome = Some(now);
                        if let Some(ws) = live.as_mut()
                            && subscribe(ws, &subs).await.is_err()
                        {
                            live = None;
                            closed = true;
                        }
                    } else if text.contains("confirm_subscription") {
                        confirms += 1;
                        if rec.close.is_some() && rec.back.is_none() {
                            if text.contains("HeartbeatChannel")
                                && refreshing.is_none()
                                && rec.refreshed.is_none()
                                && let Some(path) = &ctx.refresh_path
                            {
                                refreshing = Some(tokio::spawn(refresh(ctx.addr.clone(), path.clone(), cookie.clone())));
                            }
                            if confirms == subs.len() {
                                rec.confirmed = Some(now);
                            }
                        }
                    } else if text.contains(r#""type":"disconnect""#) {
                        live = None;
                        closed = true;
                    }
                }
                Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => {
                    live = None;
                    closed = true;
                }
                Some(Ok(_)) => {}
            },
            Event::Refreshed(result) => {
                refreshing = None;
                let (ms, status) = result.unwrap_or((0.0, 0));
                rec.refreshed = Some(now);
                rec.refresh_ms = ms;
                rec.refresh_status = status;
            }
        }
        if closed {
            if !ctx.armed.load(Ordering::Relaxed) {
                rec.early_close = true;
            }
            rec.close.get_or_insert(now);
            // A drop while getting back in voids what this socket had confirmed.
            if rec.back.is_none() {
                rec.confirmed = None;
            }
            if !disconnected {
                disconnected = true; // the close event: recordDisconnect
                disconnected_at = Some(now);
            }
            if reopen_at.is_none() {
                match ctx.policy {
                    Policy::Herd => reopen_at = Some(now + ctx.retry.mul_f64(rng.unit())),
                    Policy::Early => reopen_at = Some(now + early_wait(early_failures, &mut rng)),
                    Policy::ActionCable => {}
                }
            }
        }
        if start_attempt {
            rec.tries += 1;
            rec.first_attempt.get_or_insert(now);
            ctx.attempts.lock().unwrap().push(now);
            pending_since = now;
            pending = Some(tokio::spawn(open(ctx.addr.clone(), cookie.clone(), ctx.connect_timeout)));
        }
        if rec.back.is_none() && rec.confirmed.is_some() && (ctx.refresh_path.is_none() || rec.refreshed.is_some()) {
            rec.back = Some(Instant::now());
            ctx.back.fetch_add(1, Ordering::Relaxed);
        }
    }
    if let Some(h) = pending {
        h.abort();
    }
    rec
}

/// Probes `/up` and records each change of state: (when, up).
async fn probe(addr: String, every: Duration, changes: Arc<Mutex<Vec<(Instant, bool)>>>, mut stop: tokio::sync::watch::Receiver<bool>) {
    let mut last: Option<bool> = None;
    loop {
        let started = Instant::now();
        let up = matches!(
            tokio::time::timeout(Duration::from_secs(1), one_shot(&addr, "GET", "/up", &[], Bytes::new())).await,
            Ok(Ok(resp)) if resp.status == 200
        );
        if last != Some(up) {
            changes.lock().unwrap().push((started, up));
            last = Some(up);
        }
        tokio::select! {
            _ = tokio::time::sleep_until(started + every) => {}
            _ = stop.changed() => return,
        }
    }
}

fn summary(mut values: Vec<f64>) -> Value {
    if values.is_empty() {
        return json!(null);
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let at = |p: f64| values[((p / 100.0 * values.len() as f64) as usize).min(values.len() - 1)];
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    json!({
        "n": values.len(),
        "min": round(values[0]),
        "p10": round(at(10.0)),
        "p50": round(at(50.0)),
        "p90": round(at(90.0)),
        "p99": round(at(99.0)),
        "max": round(values[values.len() - 1]),
    })
}

fn per_second(times: impl Iterator<Item = f64>) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for t in times.filter(|t| *t >= 0.0) {
        let second = t as usize;
        if out.len() <= second {
            out.resize(second + 1, 0);
        }
        out[second] += 1;
    }
    out
}

pub(crate) async fn run(a: &Args) -> Res<Value> {
    let addr = host_port(&a.get("base"));
    let room = a.get("room");
    let clients: usize = a.num("clients", 100);
    let policy = match a.opt("policy").as_deref() {
        None | Some("actioncable") => Policy::ActionCable,
        Some("herd") => Policy::Herd,
        Some("early") => Policy::Early,
        Some(other) => return Err(format!("unknown --policy {other}").into()),
    };
    let sessions = parse_sessions(&std::fs::read_to_string(a.get("sessions"))?);
    if sessions.is_empty() {
        return Err("--sessions has no sessions".into());
    }
    let max_secs: u64 = a.num("max-secs", 180);
    let settle_secs: f64 = a.num("settle-secs", 3.0);
    let since_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis();
    let ctx = Arc::new(Ctx {
        addr: addr.clone(),
        refresh_path: (a.num("refresh", 1u8) != 0).then(|| format!("/rooms/{room}/refresh?since={since_ms}&reason=connection")),
        policy,
        retry: Duration::from_millis(a.num("retry-ms", 250)),
        early: (a.num("early-min-ms", 1000.0) / 1000.0, a.num("early-max-ms", 2000.0) / 1000.0, a.num("early-backoff", 1.0)),
        connect_timeout: Duration::from_millis(a.num("connect-timeout-ms", 2000)),
        armed: AtomicBool::new(false),
        ready: AtomicUsize::new(0),
        failed: AtomicUsize::new(0),
        back: AtomicUsize::new(0),
        attempts: Mutex::new(Vec::new()),
    });
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);

    phase("connect");
    let gate = Arc::new(tokio::sync::Semaphore::new(50));
    let handles: Vec<_> = (0..clients)
        .map(|n| {
            let (cookie, subs) = sessions[n % sessions.len()].clone();
            tokio::spawn(person(ctx.clone(), n, cookie, subs, gate.clone(), stop_rx.clone()))
        })
        .collect();
    let until = Instant::now() + Duration::from_secs(120.max(clients as u64 / 20));
    while ctx.ready.load(Ordering::Relaxed) + ctx.failed.load(Ordering::Relaxed) < clients && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let ready = ctx.ready.load(Ordering::Relaxed);
    let changes = Arc::new(Mutex::new(Vec::new()));
    let prober = tokio::spawn(probe(addr.clone(), Duration::from_millis(a.num("probe-ms", 200)), changes.clone(), stop_rx.clone()));
    tokio::time::sleep(Duration::from_secs(1)).await;
    ctx.armed.store(true, Ordering::Relaxed);
    let armed_at = Instant::now();
    let armed_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as f64;
    phase("ready");

    let until = armed_at + Duration::from_secs(max_secs);
    while ctx.back.load(Ordering::Relaxed) < ready && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    phase("back");
    tokio::time::sleep(Duration::from_secs_f64(settle_secs)).await;
    stop_tx.send(true)?;
    let mut records = Vec::with_capacity(clients);
    for h in handles {
        records.push(h.await?);
    }
    let _ = prober.await;

    let t0 = records.iter().filter(|r| !r.early_close).filter_map(|r| r.close).min();
    let Some(t0) = t0 else {
        return Ok(json!({"clients": clients, "ready": ready, "error": "no socket closed after PHASE ready: was the server restarted?"}));
    };
    let secs = |t: Instant| if t >= t0 { t.duration_since(t0).as_secs_f64() } else { -t0.duration_since(t).as_secs_f64() };
    let span = |from: Option<Instant>, to: Option<Instant>| Some(to?.duration_since(from?).as_secs_f64());
    let pick = |f: &dyn Fn(&Record) -> Option<f64>| records.iter().filter_map(f).collect::<Vec<f64>>();

    let probes = changes.lock().unwrap().clone();
    let down_at = probes.iter().find(|(t, up)| !up && *t >= armed_at).map(|(t, _)| *t);
    let up_at = down_at.and_then(|d| probes.iter().find(|(t, up)| *up && *t > d).map(|(t, _)| *t));
    let mut tries = std::collections::BTreeMap::new();
    let mut statuses = std::collections::BTreeMap::new();
    for r in records.iter().filter(|r| r.close.is_some()) {
        *tries.entry(r.tries.to_string()).or_insert(0u32) += 1;
        if r.refreshed.is_some() {
            *statuses.entry(r.refresh_status.to_string()).or_insert(0u32) += 1;
        }
    }
    if let Some(path) = a.opt("trace") {
        let ms = |t: Option<Instant>| t.map(|t| format!("{:.0}", secs(t) * 1000.0)).unwrap_or_default();
        let mut out = String::from("client\tclose\tfirst_attempt\ttries\tfailed_tries\treopens\tattempt\topened\twelcome\tconfirmed\trefreshed\trefresh_ms\trefresh_status\tback\n");
        for (n, r) in records.iter().enumerate() {
            out.push_str(&format!(
                "{n}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\t{}\t{}\n",
                ms(r.close),
                ms(r.first_attempt),
                r.tries,
                r.failed_tries,
                r.reopens,
                ms(r.attempt),
                ms(r.opened),
                ms(r.welcome),
                ms(r.confirmed),
                ms(r.refreshed),
                r.refresh_ms,
                r.refresh_status,
                ms(r.back)
            ));
        }
        std::fs::write(path, out)?;
    }
    let closes = pick(&|r| r.close.map(secs));
    let attempts = ctx.attempts.lock().unwrap().clone();
    Ok(json!({
        "clients": clients,
        "policy": a.opt("policy").unwrap_or_else(|| "actioncable".to_string()),
        "refresh": ctx.refresh_path.is_some(),
        "ready": ready,
        "failed_before_ready": ctx.failed.load(Ordering::Relaxed),
        "closed_before_ready": records.iter().filter(|r| r.early_close).count(),
        "closed": closes.len(),
        "back": records.iter().filter(|r| r.back.is_some()).count(),
        "close_spread_secs": summary(closes),
        "first_close_unix_ms": (armed_unix_ms - secs(armed_at) * 1000.0).round(),
        "server": {
            "down_at_secs": down_at.map(secs),
            "up_at_secs": up_at.map(secs),
            "first_socket_open_secs": records.iter().filter_map(|r| r.opened).min().map(secs),
        },
        // Seconds from each person's own close.
        "blackout_secs": summary(pick(&|r| span(r.close, r.back))),
        "first_attempt_secs": summary(pick(&|r| span(r.close, r.first_attempt))),
        // Seconds from the first close, on one clock.
        "back_at_secs": summary(pick(&|r| r.back.map(secs))),
        // From the start of the attempt that got in to being back: the server's share for that person.
        "work_secs": summary(pick(&|r| span(r.attempt, r.back))),
        "upgrade_secs": summary(pick(&|r| span(r.attempt, r.opened))),
        "welcome_secs": summary(pick(&|r| span(r.opened, r.welcome))),
        "confirmed_secs": summary(pick(&|r| span(r.welcome, r.confirmed))),
        "refresh_ms": summary(records.iter().filter(|r| r.refreshed.is_some()).map(|r| r.refresh_ms).collect()),
        "refresh_status": statuses,
        "tries": tries,
        "failed_tries": records.iter().map(|r| r.failed_tries as u64).sum::<u64>(),
        "reopens_of_silent_sockets": records.iter().map(|r| r.reopens as u64).sum::<u64>(),
        "attempts_per_second": per_second(attempts.iter().map(|t| secs(*t))),
        "back_per_second": per_second(records.iter().filter_map(|r| r.back.map(secs))),
    }))
}
