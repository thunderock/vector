//! POLISH-04 D-79: byte-level OSC sniffer for OSC 7 (cwd) + OSC 133 (prompt marks).
//! These OSC codes are NOT dispatched by alacritty_terminal 0.26 / vte 0.15.
//! Pattern: run a second `vte::Parser` in parallel with alacritty's feed —
//! bytes flow through alacritty unchanged; this sniffer is observer-only.

use std::path::PathBuf;

use percent_encoding::percent_decode;
use vte::Perform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Start,
    Command,
    Output,
    End,
}

#[derive(Debug, Clone)]
pub struct PromptMark {
    pub kind: PromptKind,
    pub exit_code: Option<i32>,
}

#[derive(Default, Debug)]
pub struct OscEvents {
    pub cwd_updates: Vec<PathBuf>,
    pub prompt_marks: Vec<PromptMark>,
}

#[derive(Default)]
pub struct OscSniff {
    pub events: OscEvents,
}

impl Perform for OscSniff {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        match params[0] {
            b"7" if params.len() >= 2 => {
                if let Some(path) = parse_osc7_file_url(params[1]) {
                    self.events.cwd_updates.push(path);
                }
            }
            b"133" if params.len() >= 2 => {
                let kind = match params[1].first().copied() {
                    Some(b'A') => PromptKind::Start,
                    Some(b'B') => PromptKind::Command,
                    Some(b'C') => PromptKind::Output,
                    Some(b'D') => PromptKind::End,
                    _ => return,
                };
                let exit_code = if kind == PromptKind::End && params.len() >= 3 {
                    std::str::from_utf8(params[2])
                        .ok()
                        .and_then(|s| s.parse::<i32>().ok())
                } else {
                    None
                };
                self.events
                    .prompt_marks
                    .push(PromptMark { kind, exit_code });
            }
            _ => {}
        }
    }

    fn print(&mut self, _: char) {}
    fn execute(&mut self, _: u8) {}
    fn hook(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn csi_dispatch(&mut self, _: &vte::Params, _: &[u8], _: bool, _: char) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

/// Parse `file://host/path/`. Returns None on non-local host or non-file scheme.
/// Percent-decodes the path (Pitfall 3) and tolerates non-UTF-8 paths on Unix.
fn parse_osc7_file_url(payload: &[u8]) -> Option<PathBuf> {
    let s = payload.strip_prefix(b"file://")?;
    let slash = s.iter().position(|&b| b == b'/')?;
    let host = &s[..slash];
    if !host.is_empty() && host != b"localhost" {
        return None;
    }
    let path_bytes = &s[slash..];
    let decoded = percent_decode(path_bytes).collect::<Vec<u8>>();
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(decoded).ok().map(PathBuf::from)
    }
}
