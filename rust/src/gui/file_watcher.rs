//! Filesystem watcher for `dualsense-mapper.json`. When the user edits the
//! config in an external editor (Notepad, VSCode, etc.), notify-rs fires;
//! we debounce 250ms (some editors save by writing the file 3+ times in
//! a row) and emit a single `()` to the consumer, who reloads via
//! `ConfigDoc::load`.

use anyhow::Result;
use crossbeam_channel::Sender;
use notify::{Config as NConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn spawn(path: PathBuf, tx: Sender<()>) -> Result<RecommendedWatcher> {
    let last = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
    let last_cb = last.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(ev) = res {
            let mut g = last_cb.lock().unwrap();
            if g.elapsed() > Duration::from_millis(250)
                && matches!(ev.kind, notify::EventKind::Modify(_) | notify::EventKind::Create(_))
            {
                *g = Instant::now();
                let _ = tx.send(());
            }
        }
    })?;
    watcher.configure(NConfig::default().with_poll_interval(Duration::from_millis(500)))?;
    watcher.watch(&path, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}
