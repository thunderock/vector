//! vector-tunnel-agent — Linux user-space binary. Phase 8 Wave 0 = stub.
//! Wave 1 (Plan 08-03) fills in `RelayTunnelHost` + PTY spawn + protocol loop.

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str);
    match cmd {
        Some("--reauth") => {
            eprintln!("vector-tunnel-agent: --reauth not yet implemented (Phase 8 Wave 1)");
            std::process::exit(2);
        }
        Some("--version") => {
            println!("vector-tunnel-agent {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => {
            eprintln!("vector-tunnel-agent: stub. Phase 8 Wave 1 (Plan 08-03) wires the run loop.");
            std::process::exit(2);
        }
    }
}
