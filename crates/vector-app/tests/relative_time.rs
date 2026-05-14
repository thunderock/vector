use vector_app::relative_time::{humanize, humanize_option, state_label};
use vector_codespaces::CodespaceState;

#[test]
fn just_now_under_minute() {
    assert_eq!(humanize(0), "just now");
    assert_eq!(humanize(59), "just now");
    assert_eq!(humanize(-5), "just now");
}

#[test]
fn minutes_singular_and_plural() {
    assert_eq!(humanize(60), "1 minute ago");
    assert_eq!(humanize(120), "2 minutes ago");
    assert_eq!(humanize(3599), "59 minutes ago");
}

#[test]
fn hours_days_weeks_months_years() {
    assert_eq!(humanize(3600), "1 hour ago");
    assert_eq!(humanize(7200), "2 hours ago");
    assert_eq!(humanize(86_400), "1 day ago");
    assert_eq!(humanize(2 * 86_400), "2 days ago");
    assert_eq!(humanize(604_800), "1 week ago");
    assert_eq!(humanize(2 * 604_800), "2 weeks ago");
    assert_eq!(humanize(2_592_000), "1 month ago");
    assert_eq!(humanize(6 * 2_592_000), "6 months ago");
    assert_eq!(humanize(31_536_000), "1 year ago");
    assert_eq!(humanize(2 * 31_536_000), "2 years ago");
}

#[test]
fn humanize_option_handles_none() {
    assert_eq!(humanize_option(None), "never");
    assert_eq!(humanize_option(Some(0)), "just now");
}

#[test]
fn state_label_subsumes_starting_family() {
    assert_eq!(state_label(CodespaceState::Available), "Available");
    assert_eq!(state_label(CodespaceState::Starting), "Starting");
    assert_eq!(state_label(CodespaceState::Provisioning), "Starting");
    assert_eq!(state_label(CodespaceState::Queued), "Starting");
    assert_eq!(state_label(CodespaceState::Updating), "Starting");
    assert_eq!(state_label(CodespaceState::Rebuilding), "Starting");
    assert_eq!(state_label(CodespaceState::Created), "Starting");
}

#[test]
fn state_label_subsumes_shutdown_family() {
    assert_eq!(state_label(CodespaceState::Shutdown), "Shutdown");
    assert_eq!(state_label(CodespaceState::ShuttingDown), "Shutdown");
    assert_eq!(state_label(CodespaceState::Archived), "Shutdown");
}

#[test]
fn state_label_unrecognized_renders_unknown() {
    assert_eq!(state_label(CodespaceState::Unrecognized), "Unknown");
    assert_eq!(state_label(CodespaceState::Unknown), "Unknown");
    assert_eq!(state_label(CodespaceState::Failed), "Failed");
}
