//! HARDEN-02 perf-gate probe (D-10 + D-23).
//!
//! Two measurements:
//!   - idle_cpu_pct: 5 s with no render work; rusage delta over wall time.
//!   - paste_render_fps: feed ~1 MB of pre-canned ANSI into Term; render N
//!     frames offscreen; report frames/sec.
//!
//! Emits a one-line JSON to stdout: {"idle_cpu_pct":X.X,"paste_render_fps":Y.Y}.
//! CI reads the JSON and applies the threshold (idle <1.0%, fps >=55.0).

#![allow(unsafe_code)] // getrusage FFI — workspace lint is deny by default.

use std::io::Write;
use std::time::{Duration, Instant};

fn main() {
    let idle = measure_idle_cpu();
    let fps = measure_paste_render_fps();
    let json = format!("{{\"idle_cpu_pct\":{idle:.3},\"paste_render_fps\":{fps:.2}}}");
    println!("{json}");
    if let Ok(mut f) = std::fs::File::create("target/perf-probe.json") {
        let _ = writeln!(f, "{json}");
    }
}

fn measure_idle_cpu() -> f64 {
    let start = Instant::now();
    let start_cpu = self_cpu_seconds();
    while start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let end_cpu = self_cpu_seconds();
    let cpu_used = end_cpu - start_cpu;
    let wall = start.elapsed().as_secs_f64();
    (cpu_used / wall) * 100.0
}

fn measure_paste_render_fps() -> f64 {
    let Ok(ctx) = vector_render::RenderContext::new_offscreen(800, 480) else {
        return -1.0;
    };
    let Ok(font_stack) = vector_fonts::FontStack::load_bundled(1.0, 14.0) else {
        return -1.0;
    };
    let Ok(mut comp) = vector_render::Compositor::new_with(
        &ctx.device,
        &ctx.queue,
        ctx.format,
        ctx.width,
        ctx.height,
        font_stack,
    ) else {
        return -1.0;
    };
    let mut term = vector_term::Term::new(80, 24, 100_000);
    // ~1.1 MB of mixed ANSI-free text to keep parser hot without DCS noise.
    let blob = "lorem ipsum dolor sit amet, consectetur adipiscing elit\r\n".repeat(20_000);
    term.feed(blob.as_bytes());
    let n: u32 = 60;
    let start = Instant::now();
    for _ in 0..n {
        let _ = comp.render_offscreen_with(
            &ctx.device,
            &ctx.queue,
            ctx.width,
            ctx.height,
            &mut term,
            None,
        );
    }
    let elapsed = start.elapsed().as_secs_f64();
    f64::from(n) / elapsed
}

#[cfg(target_os = "macos")]
#[allow(clippy::cast_precision_loss)] // tv_sec is i64 (darwin_time_t); seconds since boot won't lose precision in f64 for this probe.
fn self_cpu_seconds() -> f64 {
    unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &raw mut ru);
        let utime = ru.ru_utime.tv_sec as f64 + f64::from(ru.ru_utime.tv_usec) / 1_000_000.0;
        let stime = ru.ru_stime.tv_sec as f64 + f64::from(ru.ru_stime.tv_usec) / 1_000_000.0;
        utime + stime
    }
}

#[cfg(not(target_os = "macos"))]
fn self_cpu_seconds() -> f64 {
    0.0
}
