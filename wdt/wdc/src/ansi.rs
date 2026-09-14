// wdc/src/ansi.rs
//
//! The `ansiprint` macro and a check for `ANSI` sequence support.
use std::io::IsTerminal;
use std::sync::OnceLock;

#[macro_export]
macro_rules! ansiprint {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {{
        let s = format!($fmt $(, $arg)*);
        if $crate::ansi::ansi_supported() {
            println!("{s}")
        } else {
            println!("{}", $crate::ansi::strip_ansi_preserve_quotes(&s))
        }
    }};
}

pub fn ansi_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        if !std::io::stdout().is_terminal() && !std::io::stderr().is_terminal() {
            return false;
        }
        if std::env::var_os("NO_COLOR").is_some() {
            return false;
        }
        if cfg!(target_os = "windows") {
            return std::env::var("WT_SESSION").is_ok()
                || std::env::var("ConEmuANSI")
                    .map(|v| v == "ON")
                    .unwrap_or(false)
                || std::env::var("TERM_PROGRAM").is_ok();
        }
        match std::env::var("TERM") {
            Ok(term) => {
                let t = term.to_lowercase();
                t != "dumb"
                    && (t.contains("color")
                        || t.contains("xterm")
                        || t.contains("256")
                        || t.contains("linux")
                        || t.contains("ansi")
                        || t.contains("kitty")
                        || t.contains("alacritty"))
            }
            Err(_) => false,
        }
    })
}

pub fn strip_ansi_preserve_quotes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_quote = false;

    while let Some(ch) = chars.next() {
        if ch == '\'' {
            in_quote = !in_quote;
            result.push(ch);
        } else if !in_quote && ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn unicode_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();

    *SUPPORTED.get_or_init(|| {
        if !std::io::stdout().is_terminal() && !std::io::stderr().is_terminal() {
            return false;
        }

        // Windows Terminal, ConEmu and modern Windows consoles support
        // Unicode natively.
        if cfg!(target_os = "windows") {
            return std::env::var("WT_SESSION").is_ok()
                || std::env::var("ConEmuANSI")
                    .map(|v| v == "ON")
                    .unwrap_or(false)
                || std::env::var("TERM_PROGRAM").is_ok();
        }

        // On Unix-like systems we check the locale.
        let locale = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_CTYPE"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();

        locale.contains("utf-8") || locale.contains("utf8")
    })
}
