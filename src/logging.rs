/// Dual-output logging: pretty colored stderr + JSON file.
///
/// Console: human-readable, aurora-colored, stderr.
/// File:    structured JSON, `{data_dir}/logs/tailscale.log`, 10 MB cap (truncate on overflow).
///
/// File logging is skipped in stdio MCP mode to avoid polluting stdout with file‑lock noise;
/// callers that want file logging must pass `enable_file: true`.
pub mod aurora;

use std::{
    fs::{self, OpenOptions},
    io::{self, IsTerminal, Write},
    path::Path,
    sync::{Arc, Mutex},
};

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc, FormatFields, MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

/// Initialize dual logging.
///
/// - `data_dir`:    base appdata directory (e.g. `~/.tailscale-mcp`)
/// - `service`:     log file base name (e.g. `"tailscale"`)
/// - `default_level`: fallback filter string when `RUST_LOG` is unset
/// - `enable_file`: write to `{data_dir}/logs/{service}.log` (disable for stdio mode)
pub fn init(
    data_dir: &Path,
    service: &str,
    default_level: &str,
    enable_file: bool,
) -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    let use_color = should_colorize();

    let console_layer = fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(use_color)
        .with_target(false)
        .with_timer(ChronoUtc::rfc_3339())
        .with_level(true)
        .event_format(AuroraFormatter { use_color });

    if enable_file {
        let log_path = data_dir.join("logs").join(format!("{service}.log"));
        fs::create_dir_all(log_path.parent().expect("log path has parent"))?;

        // Truncate if the file is at or over the cap.
        if log_path.exists() {
            let size = fs::metadata(&log_path)?.len();
            if size >= MAX_LOG_BYTES {
                fs::write(&log_path, b"")?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| anyhow::anyhow!("cannot open log file {}: {e}", log_path.display()))?;

        let file_writer = Arc::new(Mutex::new(file));

        let file_layer = fmt::layer()
            .with_writer(MutexWriter(file_writer))
            .with_ansi(false)
            .json()
            .with_timer(ChronoUtc::rfc_3339())
            .with_current_span(false);

        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .init();
    }

    Ok(())
}

// ── color detection ───────────────────────────────────────────────────────────

fn should_colorize() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("FORCE_COLOR").is_ok() {
        return true;
    }
    io::stderr().is_terminal()
}

// ── aurora console formatter ──────────────────────────────────────────────────

/// Custom event formatter that applies aurora colors to log levels and key fields.
struct AuroraFormatter {
    use_color: bool,
}

impl<S, N> fmt::FormatEvent<S, N> for AuroraFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &fmt::FmtContext<'_, S, N>,
        mut writer: fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        // Timestamp
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        write!(writer, "{now}  ")?;

        // Level with aurora colors
        let level = *event.metadata().level();
        let level_str = if self.use_color {
            match level {
                Level::ERROR => aurora::bold(&aurora::ansi256(aurora::ERROR, "ERROR")),
                Level::WARN => aurora::bold(&aurora::ansi256(aurora::WARN, " WARN")),
                Level::INFO => " INFO".to_string(),
                Level::DEBUG => aurora::dim("DEBUG"),
                Level::TRACE => aurora::dim("TRACE"),
            }
        } else {
            match level {
                Level::ERROR => "ERROR".to_string(),
                Level::WARN => " WARN".to_string(),
                Level::INFO => " INFO".to_string(),
                Level::DEBUG => "DEBUG".to_string(),
                Level::TRACE => "TRACE".to_string(),
            }
        };
        write!(writer, "{level_str}  ")?;

        // Service name in pink
        let svc = if self.use_color {
            aurora::ansi256(aurora::SERVICE_NAME, "tailscale-mcp")
        } else {
            "tailscale-mcp".to_string()
        };
        write!(writer, "{svc}  ")?;

        // Fields (delegates to default formatter)
        ctx.format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

// ── mutex-backed file writer ──────────────────────────────────────────────────

/// A `MakeWriter` backed by a `Mutex<File>` so the file can be shared across the
/// subscriber registry without requiring `Send + Sync` on the raw `File`.
#[derive(Clone)]
struct MutexWriter(Arc<Mutex<std::fs::File>>);

impl<'a> MakeWriter<'a> for MutexWriter {
    type Writer = GuardWriter;

    fn make_writer(&'a self) -> Self::Writer {
        GuardWriter(self.0.clone())
    }
}

struct GuardWriter(Arc<Mutex<std::fs::File>>);

impl Write for GuardWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?
            .write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?
            .flush()
    }
}
