/// Aurora palette — ANSI 256 constants (matches lab/crates/lab/src/output/theme.rs exactly).
/// These are the single source of truth for console log coloring in rustscale.
pub const SERVICE_NAME: u8 = 211; // pink        (255,175,215)
pub const ACCENT_PRIMARY: u8 = 39; // bright blue (41,182,246)
#[allow(dead_code)]
pub const TEXT_MUTED: u8 = 250; // light grey  (167,188,201)
pub const SUCCESS: u8 = 115; // teal        (125,211,199)
pub const WARN: u8 = 180; // amber       (198,163,107)
pub const ERROR: u8 = 174; // muted red   (199,132,144)

/// Wrap text in ANSI 256-color escape codes if the sink is a TTY.
#[must_use]
pub fn ansi256(code: u8, text: &str) -> String {
    format!("\x1b[38;5;{code}m{text}\x1b[0m")
}

/// Bold escape wrapper.
#[must_use]
pub fn bold(text: &str) -> String {
    format!("\x1b[1m{text}\x1b[0m")
}

/// Dim escape wrapper (used for DEBUG/TRACE).
#[must_use]
pub fn dim(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}
