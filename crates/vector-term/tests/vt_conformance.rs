//! HARDEN-02 VT conformance corpus. One module per ROADMAP scenario.
//! Each scenario maps to a PITFALLS.md item — see module docs.

#[path = "vt_conformance/alt_screen_1049.rs"]
mod alt_screen_1049;
#[path = "vt_conformance/bracketed_paste.rs"]
mod bracketed_paste;
#[path = "vt_conformance/decscusr.rs"]
mod decscusr;
#[path = "vt_conformance/ed_el_erase.rs"]
mod ed_el_erase;
#[path = "vt_conformance/mouse_1006.rs"]
mod mouse_1006;
#[path = "vt_conformance/osc52_round_trip.rs"]
mod osc52_round_trip;
#[path = "vt_conformance/scroll_regions.rs"]
mod scroll_regions;
#[path = "vt_conformance/tab_stops.rs"]
mod tab_stops;
