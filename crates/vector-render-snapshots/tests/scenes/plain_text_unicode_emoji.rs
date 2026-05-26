//! HARDEN-01 D-02 scene (a): plain text + mixed Unicode + emoji.

#[path = "../common/mod.rs"]
mod common;

use common::{diff_or_panic, render_scene};

#[test]
fn plain_text_unicode_emoji() {
    let Some(img) = render_scene(800, 480, 80, 24, |term| {
        term.feed(b"hello, world\r\n");
        // Japanese: 日本語テスト
        term.feed("\u{65e5}\u{672c}\u{8a9e}\u{30c6}\u{30b9}\u{30c8}\r\n".as_bytes());
        // Emoji: 🎉 ✨ 🚀
        term.feed("emoji: \u{1f389} \u{2728} \u{1f680}\r\n".as_bytes());
        term.feed(b"rust: let x = 1;\r\n");
    }) else {
        eprintln!("SKIP: no Metal adapter");
        return;
    };
    diff_or_panic(&img, "plain_text_unicode_emoji");
}
