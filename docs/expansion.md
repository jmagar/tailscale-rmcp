# syslog-mcp expansion · session bootstrap

> Briefing doc to load at session start. Captures fleet topology, current
> log ingest paths, planned expansion, drop-in configs, and the
> architecture decisions made en route. Implementation work is plotted at
> the bottom.

---

## 0 · context

- **owner**: jacob · homelab `tootie.tv` · tailscale-meshed
- **existing**: `syslog-mcp` (Rust/Axum + SQLite FTS5, custom MCP tools) ingesting
  - syslog/journald from limited hosts
  - container stdout via `dockersocketproxy` (not the docker syslog log driver — chosen for container-startup resilience and to avoid per-host fluent agents)
- **goal**: expand ingestion across the full fleet, capture missing log streams (nginx/authelia/fail2ban/adguard/zfs/smartd/ai), add OTLP HTTP receiver to absorb claude code & codex telemetry, host on `shart`
- **adjacent**: `axon` (Rust RAG pipeline · TEI Qwen3-Embedding-0.6B · Qdrant · Postgres · Redis · spider.rs) handles documentation + transcript content. Logs and docs stay in separate corpora.

---

## 1 · fleet

| host | os | role | hardware | storage | net |
|---|---|---|---|---|---|
| **tootie** | Unraid 7.x | media + VM host | — | 3×2TB raidz1 | 2.5GbE |
| **dookie** | Ubuntu 25 (VM on tootie) | dev / AI / GPU | RTX 4070 + nvme + 60GB passthrough | ZFS (passthrough nvme) | 2.5GbE (host) |
| **squirts** | Ubuntu 25 | mission-critical services | i3 NUC, UPS-backed | ZFS, hourly snaps | 2.5GbE |
| **shart** | Unraid 7.x | backup sink + future syslog-mcp host | — | 2×8TB mirror (spinners) | 2.5GbE |
| **steamy** | Win11 + WSL Ubuntu 25 | primary workstation | i5 11th, 48GB, RTX 3050, 2TB nvme | — | 2.5GbE |
| **vivobook** | Win11 + WSL Ubuntu 25 | mobile, parsec → steamy | i3 13th, 24GB, 1TB nvme | — | wifi 7 |

**Network edge:** ATT BGW-320 → UCG-Max → 2.5GbE PoE switch + U7 Pro AP. Tailscale flat mesh across all 6 nodes. SWAG terminates `*.tootie.tv` on squirts; non-public subdomains gated by Authelia + Duo 2FA.

**Snapshot chain:** tootie/dookie/squirts → shart (hourly) → Google Drive (5TB offsite). Squirts' ~30GB critical set replicated to every node. dookie repos additionally to tootie.

**Workflow chain:** vivobook (parsec) → steamy (zed remote) → dookie (compute).

**Codename naming:** scatological theme; vivobook is the lone holdout.

---

## 2 · current ingest paths

| source | mechanism | status |
|---|---|---|
| tootie syslog | Unraid Settings → Syslog Server | ✅ active |
| shart syslog | Unraid Settings → Syslog Server | ✅ active |
| Docker container stdout (all hosts) | dockersocketproxy → syslog-mcp | ✅ active |
| ubuntu/WSL host syslog | rsyslog default | ⚠️ partial (no journald) |
| nginx access/error (SWAG) | none | ❌ missing |
| Authelia auth events | none — writing to file | ❌ missing |
| fail2ban (in SWAG) | none | ❌ missing |
| AdGuard query log | none | ❌ missing |
| Vaultwarden auth | via container stdout | ⚠️ verify |
| ZED (zfs events) | already in syslog default | ⚠️ verify by tag |
| smartd | already in syslog default | ⚠️ verify by tag |
| auditd | killed via `audit=0` on dookie (kernel cmdline) | ✅ noise eliminated |
| claude code OTel | none | ❌ missing |
| codex OTel | none | ❌ missing |
| claude/codex `.jsonl` transcripts | none | ❌ missing |
| libvirt (dookie) on tootie | none | ❌ TBD (Unraid API vs imfile) |

**Why dockersocketproxy and not the syslog log driver:** if the syslog server is unreachable at container start, the syslog log driver will refuse to start the container. Pulling logs from the docker socket (read-only proxy) decouples container lifecycle from syslog-mcp availability. Trade-off: claude needs the proxy reachable to ingest, but containers run regardless.

---

## 3 · planned ingest expansion

### 3.1 host roles for log work

| host | drop-ins | notes |
|---|---|---|
| **tootie** | none (Unraid native covers it) | `imfile` for libvirt only if Unraid API insufficient — needs User Scripts plugin to persist in `/etc/rsyslog.d/` |
| **shart** | none | future syslog-mcp host |
| **squirts** | imjournal · swag · authelia · adguard | the special one — all SWAG/auth/dns lives here |
| **dookie** | imjournal · ai-transcripts | + claude/codex jsonls |
| **steamy-wsl** | imjournal · ai-transcripts | + claude/codex jsonls |
| **vivobook-wsl** | imjournal · ai-transcripts | + claude/codex jsonls |

### 3.2 modern Ubuntu / journald gap

Ubuntu 22.04+ writes nearly everything to journald. Default rsyslog only sees what `imuxsock`/`imklog` catch + whatever journald forwards via `ForwardToSyslog=`. Most systemd services log to stderr → journald and never touch `/dev/log`. Without `imjournal`, the rsyslog stream from these hosts is sparse: cron, sshd, kernel, sudo. **`imjournal` is mandatory on all Ubuntu/WSL hosts.**

### 3.3 ZED — already on by default

OpenZFS ships `all-syslog.sh` ZEDLET enabled at install. Logs at `daemon.notice` with tag `zed`. Quick verification:

```sql
SELECT host, count(*) FROM logs WHERE tag = 'zed' GROUP BY host;
```

If empty, check `/etc/zfs/zed.d/all-syslog.sh` symlink and `systemctl status zfs-zed`. Tune in `/etc/zfs/zed.d/zed.rc`:

```bash
ZED_SYSLOG_SUBCLASS_EXCLUDE="history_event"   # drop zpool history spam
ZED_SYSLOG_PRIORITY="daemon.notice"
ZED_SYSLOG_TAG="zed"
```

### 3.4 smartd — already on by default

Native syslog out of the box, `daemon.warning`, tag `smartd`. Just confirm `smartmontools` installed and `smartd` running on each Linux host.

### 3.5 BGW-320 — skip

ATT residential gateway has no useful remote syslog. UCG-Max behind it is doing the security work; ATT box is just NAT/PPPoE. Not worth the time investment.

---

## 4 · architecture decisions

### 4.1 logs vs transcripts vs docs — three corpora, one query model

| corpus | store | search | source |
|---|---|---|---|
| event stream | syslog-mcp (FTS5) | exact / time-window / host filter | rsyslog + dockersocketproxy + OTLP |
| AI conversation content | axon (Qdrant + TEI) | semantic recall | claude/codex `.jsonl` |
| reference docs | axon (Qdrant + TEI [+ FTS5 for hybrid/RRF]) | semantic + sparse hybrid | spider.rs crawler |

**Don't ingest crawled documentation into syslog-mcp.** Different data model (reference vs event), different query patterns (topic vs time/host), pollutes log search with stale doc snippets.

**RRF clarification:** RRF fuses ranked lists from dense + sparse indexes over the **same corpus** (same chunk IDs). For axon hybrid search, FTS5 lives next to Qdrant inside axon, keyed by chunk_id. Or use Qdrant native sparse vectors (BM42/SPLADE) and skip the parallel FTS5 index entirely.

### 4.2 OTel ingestion — build into syslog-mcp, don't deploy a collector

**Rationale:**
- One less container, one less moving part (stated preference)
- Keeps "anyone can spin this up" goal intact (no OTel collector dep)
- OTLP/HTTP is trivial protobuf; existing axum + receiver muscle covers it
- Single FTS5 index across syslog + container stdout + claude/codex telemetry → cross-stream correlation works

**Scope:**
- ✅ logs (`/v1/logs`) — translate `LogRecord` → existing log shape
- ✅ traces (`/v1/traces`) — flatten spans into rows tagged with `trace_id` + `span_id`
- ❌ metrics (`/v1/metrics`) — return 404/unsupported; metrics belong in Prom/VictoriaMetrics, not FTS5
- ❌ gRPC — HTTP only unless something specifically needs it

**When to introduce a real collector:** once fanout to multiple backends (Prom + syslog-mcp + Loki) becomes necessary. YAGNI until then.

### 4.3 claude/codex transcript routing

```
claude/codex JSONLs ──► axon (semantic recall via embeddings)
                  │
                  └──► imfile ──► syslog-mcp (filter by host/time)

claude/codex OTel events ──► OTLP HTTP ──► syslog-mcp (api_request, tool_result, prompts-by-length)
```

Two destinations, two query modes:
- **axon**: "the conversation about audit backlog" (semantic)
- **syslog-mcp**: "every claude session on dookie last Tuesday between 2-4am" (structured filter + time)
- **cross-corpora**: correlate claude activity with smartd warnings on the same host & time window via syslog-mcp JOIN

### 4.4 Authelia decision

Authelia is currently writing to a file (`log.file_path` set), not stdout — that's why container stdout ingestion misses it. Two paths:

1. Remove `log.file_path` from authelia config → flows through dockersocketproxy
2. Leave file in place, point `imfile` at it (chosen — minimizes blast radius on a working auth stack)

Authelia logs are structured (`time=... level=... msg=...`). Parse `level` server-side at ingest to set proper severity rather than hardcoding `info`.

### 4.5 AdGuard query volume warning

Query log gets **loud** — every DNS query from every device. Tens of thousands per day. Worth it for phone-home detection but:
- Filter at ingest (drop `allowed` queries, keep blocked + filtered)
- Or partition into a dedicated table/index in syslog-mcp so it doesn't drown signal
- Decide retention separately from other tags

### 4.6 hosting

`syslog-mcp` runs on **shart**:
- Least loaded box in the fleet (backup-only)
- Mirror pool has 8TB headroom
- DB lands in the existing zfs send chain → automatic offsite via gdrive
- Co-located with future OTel receiver work — single container, single endpoint

---

## 5 · syslog-mcp enhancements

### 5.1 OTLP HTTP receiver

**Crate additions:**
```toml
opentelemetry-proto = { version = "*", features = ["gen-tonic-messages", "logs", "trace"] }
prost = "*"
```

**Routes:**
- `POST /v1/logs` — `ExportLogsServiceRequest` → flatten → existing FTS5 ingest
- `POST /v1/traces` — `ExportTraceServiceRequest` → flatten spans → log rows tagged with `trace_id`/`span_id`
- `POST /v1/metrics` — return 404/unsupported; do not false-ack dropped metrics

**LogRecord field mapping:**
| OTLP field | syslog-mcp column |
|---|---|
| `time_unix_nano` | timestamp |
| `severity_number` | severity |
| `body` (AnyValue) | message |
| `resource.attributes["host.name"]` | host |
| `resource.attributes["service.name"]` | program/tag |
| `attributes` | flatten → JSON column or structured fields |
| `resource.attributes["service.version"]` | metadata JSON |

**Sketch:**
```rust
async fn otlp_logs_handler(
    State(db): State<Db>,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let req = ExportLogsServiceRequest::decode(body)?;

    for resource_logs in req.resource_logs {
        let resource_attrs = flatten_kv(&resource_logs.resource);

        for scope_logs in resource_logs.scope_logs {
            for log in scope_logs.log_records {
                let row = LogRow {
                    timestamp_ns: log.time_unix_nano as i64,
                    host: resource_attrs.get("host.name").cloned(),
                    program: resource_attrs.get("service.name").cloned(),
                    severity: severity_from_number(log.severity_number),
                    message: extract_body(&log.body),
                    attributes: flatten_kv_to_json(&log.attributes),
                };
                db.insert_log(row).await?;
            }
        }
    }
    Ok(StatusCode::OK)
}
```

### 5.2 ingest-side enrichment

Most-bang-for-buck items to do at ingest, not query time:

- **Authelia severity parsing** — implemented; extract `level=` from structured body, override severity column
- **AdGuard tag classification** — implemented; parse JSON line, set `tag` to `adguard-blocked` / `adguard-allowed` / `adguard-rewrite`
- **claude/codex transcript metadata** — implemented; pull `project` and `tool` from `imfile` source filename, JSON body, or OTel attributes (e.g. `session.id`, `project.path`) into dedicated columns; query via `syslog sessions`
- **Multi-line glomming** — fail2ban + authelia panics need `startmsg.regex` discipline (already in drop-ins below)

### 5.3 retention

Don't filter at rsyslog — let it all in, retain selectively:

- **30 days** default
- **7 days** for `adguard-allowed`
- **90 days** for `tag IN ('zed', 'smartd', 'authelia', 'fail2ban', 'kern.*')`
- **forever (or until disk pressure)** for `severity >= err`

Tag-based retention is cheap: nightly job with `DELETE FROM logs WHERE tag = ? AND timestamp < ?` per rule.

### 5.4 message size

Bump `$MaxMessageSize 256k` on rsyslog hosts that ingest claude/codex jsonls — tool results and large diffs blow past the 8KB default and get truncated.

---

## 6 · drop-ins (deploy)

All drop-ins live in `/etc/rsyslog.d/`. Validate any change with `rsyslogd -N1` before reload.

### 6.1 imjournal — all Ubuntu/WSL hosts (dookie, steamy-wsl, vivobook-wsl, squirts)

```conf
# /etc/rsyslog.d/10-imjournal.conf
module(load="imjournal"
       StateFile="/var/spool/rsyslog/imjournal.state"
       Ratelimit.Interval="0"
       Ratelimit.Burst="0")
```

WSL prerequisite — `/etc/wsl.conf`:
```ini
[boot]
systemd=true
```

### 6.2 squirts only — SWAG (nginx + fail2ban)

```conf
# /etc/rsyslog.d/30-swag.conf
$MaxMessageSize 64k
module(load="imfile")

input(type="imfile"
      File="/path/to/swag/appdata/log/nginx/access.log"
      Tag="swag-access"
      Facility="local4"
      Severity="info"
      PersistStateInterval="100")

input(type="imfile"
      File="/path/to/swag/appdata/log/nginx/error.log"
      Tag="swag-error"
      Facility="local4"
      Severity="warning"
      PersistStateInterval="100")

input(type="imfile"
      File="/path/to/swag/appdata/log/fail2ban/fail2ban.log"
      Tag="fail2ban"
      Facility="local5"
      Severity="info"
      startmsg.regex="^[0-9]{4}-[0-9]{2}-[0-9]{2}"
      PersistStateInterval="100")
```

> ⚠️ Replace `/path/to/swag/appdata` with the actual mount path on squirts.

### 6.3 squirts only — Authelia

```conf
# /etc/rsyslog.d/35-authelia.conf
module(load="imfile")

input(type="imfile"
      File="/path/to/authelia/config/authelia.log"
      Tag="authelia"
      Facility="local5"
      Severity="info"
      startmsg.regex="^time="
      PersistStateInterval="100")
```

### 6.4 squirts only — AdGuard

```conf
# /etc/rsyslog.d/36-adguard.conf
$MaxMessageSize 32k
module(load="imfile")

input(type="imfile"
      File="/path/to/adguard/work/data/querylog.json"
      Tag="adguard-query"
      Facility="local6"
      Severity="info"
      PersistStateInterval="500")
```

### 6.5 dookie + steamy-wsl + vivobook-wsl — claude/codex transcripts

```conf
# /etc/rsyslog.d/40-ai-transcripts.conf
$MaxMessageSize 256k
module(load="imfile")

input(type="imfile"
      File="/home/jacob/.claude/projects/*/*.jsonl"
      Tag="claude-transcript"
      Facility="local7"
      Severity="info"
      addMetadata="on"
      PersistStateInterval="50")

input(type="imfile"
      File="/home/jacob/.codex/sessions/*.jsonl"
      Tag="codex-transcript"
      Facility="local7"
      Severity="info"
      addMetadata="on"
      PersistStateInterval="50")
```

### 6.6 unraid persistence

Standard `/etc/rsyslog.d/` doesn't survive reboot on Unraid (overlay fs). For tootie if libvirt imfile is needed:

- Option A — User Scripts plugin → run at array start: copy drop-in into `/etc/rsyslog.d/`, `kill -HUP $(cat /var/run/rsyslogd.pid)`
- Option B — append same logic to `/boot/config/go`

For tootie/shart base syslog: just use Settings → Syslog Server (native, persistent, no fuss).

### 6.7 claude code & codex OTel config

**Claude Code** (`~/.claude/settings.json` on dookie/steamy-wsl/vivobook-wsl):
```json
{
  "env": {
    "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
    "OTEL_METRICS_EXPORTER": "otlp",
    "OTEL_LOGS_EXPORTER": "otlp",
    "OTEL_EXPORTER_OTLP_PROTOCOL": "http/protobuf",
    "OTEL_EXPORTER_OTLP_ENDPOINT": "http://shart.tailnet:4318",
    "OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE": "cumulative",
    "OTEL_METRIC_EXPORT_INTERVAL": "10000",
    "OTEL_LOGS_EXPORT_INTERVAL": "5000",
    "OTEL_LOG_USER_PROMPTS": "1",
    "OTEL_LOG_TOOL_DETAILS": "1"
  }
}
```

**Codex** (`~/.codex/config.toml`):
```toml
[otel]
environment = "homelab"
log_user_prompt = true
exporter = { otlp-http = { endpoint = "http://shart.tailnet:4318/v1/logs", protocol = "binary" } }
trace_exporter = { otlp-http = { endpoint = "http://shart.tailnet:4318/v1/traces", protocol = "binary" } }
```

> ⚠️ Codex OTel coverage is incomplete — `codex exec` emits no metrics, `codex mcp-server` emits nothing. Transcript ingestion via imfile is the safety net.

---

## 7 · open questions

- **libvirt logs from tootie** — try Unraid API first; fall back to imfile + User Scripts plugin if API doesn't expose them cleanly
- **`SELECT host, count(*) FROM logs WHERE tag IN ('zed','smartd','kern')` baseline** — confirm what's already arriving before adding drop-ins
- **Authelia stdout switch** — keep file-based (current plan) or strip `log.file_path` and use container stdout? Either works; file path is lower-risk to a live auth stack
- **Vaultwarden** — verify access events present in container stdout before deciding on imfile
- **AdGuard query log retention** — how long do "allowed" queries stay? Default to 7d unless there's a reason to keep longer
- **`adguard-blocked` separate tag** — split at ingest by parsing `result_reason`/`status` field?
- **trace handling** — flatten spans into rows now, or store separately? Lean toward flatten + tag with `trace_id`

---

## 8 · implementation order

Do **not** land this all at once. Each step should run for a day or two before adding the next, so volume/cost/value of each source is observable in isolation.

1. **Stand up syslog-mcp container on shart** — bind mount on mirror pool, expose on tailscale
2. **Point Unraid Settings → Syslog Server** on tootie + shart at the new endpoint
3. **Deploy `10-imjournal.conf`** to dookie, squirts, steamy-wsl, vivobook-wsl. Validate volume baseline.
4. **Verify ZED + smartd already arriving** by tag query
5. **Build OTLP HTTP receiver** in syslog-mcp (`/v1/logs`, `/v1/traces`; reject `/v1/metrics`)
6. **Configure claude code OTel** on dookie first, then steamy-wsl, then vivobook-wsl
7. **Configure codex OTel** same order
8. **Deploy `40-ai-transcripts.conf`** to capture jsonls (axon already has them; this is for cross-correlation in syslog-mcp)
9. **Deploy `35-authelia.conf`** on squirts — first specialty source
10. **Deploy `30-swag.conf`** on squirts
11. **Deploy `36-adguard.conf`** on squirts last (highest volume — want everything else stable first)
12. **Tag-based retention rules** — nightly job
13. **Ingest-side enrichment** — authelia severity, adguard tag splitting

---

## 9 · references

- [rsyslog imfile](https://www.rsyslog.com/doc/configuration/modules/imfile.html)
- [rsyslog imjournal](https://www.rsyslog.com/doc/configuration/modules/imjournal.html)
- [RFC 5424 syslog format](https://datatracker.ietf.org/doc/html/rfc5424)
- [openzfs ZED docs](https://openzfs.github.io/openzfs-docs/man/master/8/zed.8.html) · [zed.rc reference](https://github.com/openzfs/zfs/blob/master/cmd/zed/zed.d/zed.rc)
- [Authelia logging config](https://www.authelia.com/configuration/miscellaneous/logging/)
- [Claude Code monitoring (OTel)](https://code.claude.com/docs/en/monitoring-usage)
- [Codex advanced config (OTel)](https://developers.openai.com/codex/config-advanced) · [config reference](https://developers.openai.com/codex/config-reference)
- [opentelemetry-proto crate](https://docs.rs/opentelemetry-proto)
- [OTLP/HTTP spec](https://opentelemetry.io/docs/specs/otlp/#otlphttp)
- [Qdrant hybrid search](https://qdrant.tech/articles/hybrid-search/) (relevant to axon, not syslog-mcp)

---

## 10 · ground rules for this work

- Stack: Rust (axum, tokio, sqlx) for syslog-mcp · TypeScript / Python for tooling around it
- Modular crates if syslog-mcp grows — `syslog-mcp-core`, `syslog-mcp-otlp`, `syslog-mcp-mcp` (mcp tool surface), `syslog-mcp-bin`
- Monorepo-friendly: this likely belongs in the consolidating MCP plugin monorepo (git subtree)
- No agent-foo: don't introduce abstractions for one consumer. OTLP receiver, FTS5 ingest, MCP tool surface — three concrete things, no plugin trait
- Self-hosted only — no SaaS observability, no vendored exporters
- Data is small in absolute terms; correctness > clever tiering. SQLite with FTS5 is the right call until proven otherwise
