//! UI-SPEC §6.5 relative-time formatter + §6.4 CodespaceState → display label.

use vector_codespaces::CodespaceState;

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

/// UI-SPEC §6.4: API CodespaceState → display label.
#[must_use]
pub fn state_label(state: CodespaceState) -> &'static str {
    match state {
        CodespaceState::Available => "Available",
        CodespaceState::Starting
        | CodespaceState::Provisioning
        | CodespaceState::Queued
        | CodespaceState::Updating
        | CodespaceState::Rebuilding
        | CodespaceState::Created => "Starting",
        CodespaceState::Shutdown
        | CodespaceState::ShuttingDown
        | CodespaceState::Archived => "Shutdown",
        CodespaceState::Failed => "Failed",
        CodespaceState::Unknown | CodespaceState::Unrecognized => "Unknown",
    }
}

/// State badge RGBA — UI-SPEC §4.1 (dark theme).
#[must_use]
pub fn state_color(state: CodespaceState) -> [f32; 4] {
    match state_label(state) {
        "Available" => [0.188, 0.820, 0.345, 1.0], // #30d158
        "Starting" => [1.000, 0.839, 0.039, 1.0],  // #ffd60a
        "Failed" => [1.000, 0.271, 0.227, 1.0],    // #ff453a
        _ => [0.557, 0.557, 0.576, 1.0],           // #8e8e93
    }
}
