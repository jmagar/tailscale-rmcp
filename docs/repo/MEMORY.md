# Memory Files -- syslog-mcp

Claude Code memory system for persistent knowledge across sessions.

## What is memory

Claude Code stores session learnings in memory files under `.claude/projects/`. These persist across conversations and help maintain context about project decisions, patterns, and gotchas.

## Key memory topics for syslog-mcp

| Topic | Key facts |
| --- | --- |
| Port choice | 1514 not 514 -- avoids root/CAP_NET_BIND_SERVICE; iptables redirect for devices that cannot be reconfigured |
| WAL mode | SQLite runs in WAL mode; safe backup requires checkpoint or `.backup` method |
| FTS5 hyphen | Hyphen is the FTS5 NOT operator; search hyphenated terms with phrase syntax: `"smoke-test"` |
| Storage budget | Two-threshold hysteresis system; trigger/recovery thresholds prevent oscillation |
| source_ip trust | hostname field is spoofable via UDP; source_ip is the only network-verified identity |
| Batch writer | In-memory mpsc channel; no durable WAL; persistent failures cause data loss after 10K buffer |
| FTS5 phantoms | Deleted logs leave phantom FTS5 entries; cleaned by periodic incremental merge; query path unaffected |
| CEF parsing | UniFi CEF messages extract hostname from UNIFIdeviceName extension, not syslog header |
| correlate_events cap | limit parameter capped at 999 (not 1000) due to truncation sentinel needing limit+1 |
| Docker config | config.toml not in image; defaults + env vars only in containers |
| OCI publishing | Uses GHCR + crates.io (not PyPI/npm) |

## Memory file location

```
~/.claude/projects/-home-jmagar-workspace-syslog-mcp/memory/
```

## See also

- [../../CLAUDE.md](../../CLAUDE.md) -- project instructions (includes gotchas section)
