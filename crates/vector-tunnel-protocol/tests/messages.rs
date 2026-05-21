//! Wave 0 codec tests for `AgentMessage` (D-12..D-15).

use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

#[test]
fn open_pty_round_trips_byte_identical() {
    let msg = AgentMessage::OpenPty {
        protocol_version: 1,
        rows: 24,
        cols: 80,
        shell: None,
    };
    let s = serde_json::to_string(&msg).unwrap();
    let back: AgentMessage = serde_json::from_str(&s).unwrap();
    let s2 = serde_json::to_string(&back).unwrap();
    assert_eq!(s, s2);
    assert_eq!(msg, back);
}

#[test]
fn data_bytes_serialize_as_base64_string() {
    let msg = AgentMessage::Data {
        session: "s1".into(),
        bytes: vec![0xc3, 0xa9],
    };
    let s = serde_json::to_string(&msg).unwrap();
    // base64(STANDARD) of [0xc3, 0xa9] is "w6k=" — must be a STRING, not an array.
    assert!(
        s.contains("\"w6k=\""),
        "expected base64 string \"w6k=\" in {s}"
    );
    assert!(!s.contains("195"), "must not serialize as byte array: {s}");
}

#[test]
fn unknown_op_deserializes_to_unknown_no_panic() {
    let raw = r#"{"op":"future_thing","extra":42}"#;
    let m: AgentMessage = serde_json::from_str(raw).expect("unknown op must not panic");
    assert!(matches!(m, AgentMessage::Unknown));
}

#[test]
fn protocol_version_is_one() {
    let v: u32 = PROTOCOL_VERSION;
    assert_eq!(v, 1);
}
