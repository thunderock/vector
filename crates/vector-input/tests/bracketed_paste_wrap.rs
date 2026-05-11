//! Plan 03-04 Task 1: bracketed-paste wrap (D-53).

use vector_input::wrap_bracketed_paste;

#[test]
fn wraps_plain_ascii() {
    let out = wrap_bracketed_paste("hello");
    assert_eq!(&out[..6], b"\x1b[200~");
    assert_eq!(&out[6..11], b"hello");
    assert_eq!(&out[11..], b"\x1b[201~");
}

#[test]
fn empty_string_has_only_markers() {
    let out = wrap_bracketed_paste("");
    assert_eq!(out, b"\x1b[200~\x1b[201~".to_vec());
    assert_eq!(out.len(), 12);
}

#[test]
fn normalizes_crlf_to_lf() {
    let out = wrap_bracketed_paste("a\r\nb");
    assert!(out.windows(3).any(|w| w == b"a\nb"));
    assert!(!out.windows(2).any(|w| w == b"\r\n"));
}

#[test]
fn normalizes_lone_cr_to_lf() {
    let out = wrap_bracketed_paste("a\rb");
    assert!(out.contains(&b'\n'));
    assert!(!out.contains(&b'\r'));
}
