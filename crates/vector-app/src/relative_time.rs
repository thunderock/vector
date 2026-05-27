//! UI-SPEC §6.5 relative-time formatter. Used by the DevTunnels picker row
//! template (Plan 08-05) and any future "X ago" labels.

pub fn humanize(elapsed_secs: i64) -> String {
    if elapsed_secs < 60 {
        return "just now".to_string();
    }
    let (n, unit) = if elapsed_secs < 3_600 {
        (elapsed_secs / 60, "minute")
    } else if elapsed_secs < 86_400 {
        (elapsed_secs / 3_600, "hour")
    } else if elapsed_secs < 604_800 {
        (elapsed_secs / 86_400, "day")
    } else if elapsed_secs < 2_592_000 {
        (elapsed_secs / 604_800, "week")
    } else if elapsed_secs < 31_536_000 {
        (elapsed_secs / 2_592_000, "month")
    } else {
        (elapsed_secs / 31_536_000, "year")
    };
    let plural = if n == 1 { "" } else { "s" };
    format!("{n} {unit}{plural} ago")
}

pub fn humanize_option(elapsed: Option<i64>) -> String {
    match elapsed {
        Some(n) => humanize(n),
        None => "never".to_string(),
    }
}
