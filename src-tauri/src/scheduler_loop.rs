use std::thread;
use std::time::Duration;

use chrono::Local;
use tauri::{AppHandle, Manager};
use yub_core::next_fire;

use crate::popup::{self, Popup};
use crate::state::AppState;

/// Re-check the clock at least this often, so a laptop waking from sleep
/// doesn't wait out a stale timer.
const MAX_NAP: Duration = Duration::from_secs(30);
const STALE: chrono::Duration = chrono::Duration::minutes(20);

pub fn spawn(app: AppHandle) {
    thread::spawn(move || loop {
        let state = app.state::<AppState>();
        let now = Local::now().naive_local();
        let next = next_fire(now, &state.settings());

        let nap = match next {
            Some(f) if f.at <= now => Duration::ZERO,
            Some(f) => (f.at - now).to_std().unwrap_or(MAX_NAP).min(MAX_NAP),
            None => MAX_NAP,
        };

        // Sleep, but wake early if settings change.
        let gen = state.generation();
        let start = *gen;
        let (gen, timeout) = state.changed.wait_timeout_while(gen, nap, |g| *g == start).unwrap_or_else(|e| e.into_inner());
        drop(gen);
        if !timeout.timed_out() {
            continue; // settings changed: re-plan
        }

        if let Some(f) = next {
            let late = Local::now().naive_local() - f.at;
            // Missed by more than STALE (e.g. the laptop slept)? Skip it.
            if late >= chrono::Duration::zero() && late < STALE {
                if app.state::<Popup>().foreground_is_fullscreen() {
                    continue; // never interrupt a meeting or presentation
                }
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || popup::show(&handle, f.kind));
            }
        }
    });
}
