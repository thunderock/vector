//! POLISH-04 / D-78: OSC 8 hyperlink scheme allowlist + per-row run grouping.
//!
//! Allowed schemes: https, http, mailto, file://. Everything else is logged
//! at info and ignored (Pitfall: security).
//!
//! Grouping rule (Pitfall 4):
//! - When OSC 8 carries `id=foo`: group all cells sharing that id.
//! - When OSC 8 is anonymous: group by `uri` + contiguity (adjacent cells with
//!   the same uri belong to one run; gap or different uri starts a new run).

pub fn is_allowed_scheme(uri: &str) -> bool {
    const ALLOWED: &[&str] = &["https://", "http://", "mailto:", "file://"];
    ALLOWED.iter().any(|p| uri.starts_with(p))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperlinkRun {
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize, // exclusive
    pub uri: String,
    pub id: Option<String>,
}

/// Walk a row's cells, producing contiguous hyperlink runs per the grouping rule.
/// `cells` yields `(col, Option<(uri, Option<id>)>)`.
pub fn group_row<I>(row: usize, cells: I) -> Vec<HyperlinkRun>
where
    I: IntoIterator<Item = (usize, Option<(String, Option<String>)>)>,
{
    let mut runs: Vec<HyperlinkRun> = Vec::new();
    let mut current: Option<HyperlinkRun> = None;
    for (col, link) in cells {
        match (current.as_mut(), link) {
            (None, None) => {}
            (Some(_), None) => {
                runs.push(current.take().unwrap());
            }
            (None, Some((uri, id))) => {
                if !is_allowed_scheme(&uri) {
                    tracing::info!(uri = %uri, "OSC 8 scheme not in allowlist; ignored");
                    continue;
                }
                current = Some(HyperlinkRun {
                    row,
                    col_start: col,
                    col_end: col + 1,
                    uri,
                    id,
                });
            }
            (Some(run), Some((uri, id))) => {
                let same = match (&run.id, &id) {
                    (Some(a), Some(b)) => a == b,
                    (None, None) => run.uri == uri && run.col_end == col,
                    _ => false,
                };
                if same {
                    run.col_end = col + 1;
                } else {
                    runs.push(current.take().unwrap());
                    if is_allowed_scheme(&uri) {
                        current = Some(HyperlinkRun {
                            row,
                            col_start: col,
                            col_end: col + 1,
                            uri,
                            id,
                        });
                    } else {
                        tracing::info!(uri = %uri, "OSC 8 scheme not in allowlist; ignored");
                    }
                }
            }
        }
    }
    if let Some(r) = current {
        runs.push(r);
    }
    runs
}
