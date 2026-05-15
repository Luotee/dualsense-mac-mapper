use anyhow::{Context, Result};
use clap::{ArgAction, Parser};
use dualsense_mapper::app;
use dualsense_mapper::config::Config;
use dualsense_mapper::gamepad::{GamepadEvent, GamepadSource};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "dualsense-mapper", version, about)]
struct Cli {
    /// Path to config.json. Defaults to <config_dir>/dualsense-mapper/config.json.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Validate the config and exit.
    #[arg(long, action = ArgAction::SetTrue)]
    validate: bool,

    /// Print every synthesized action instead of pressing real keys.
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,

    /// Connect to the first gamepad and print events with config labels. Exits on Ctrl-C.
    #[arg(long, action = ArgAction::SetTrue)]
    list_buttons: bool,

    /// Verbose logging.
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("dualsense_mapper=debug,info")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cfg_path = resolve_config_path(cli.config.as_deref())?;

    if cli.list_buttons {
        // For list-buttons, missing config is OK — show "<no label>".
        let cfg = Config::load_from_path(&cfg_path).ok();
        return list_buttons(cfg);
    }

    // Every other path needs a valid config; if it doesn't exist, try first-run copy.
    if !cfg_path.exists() {
        first_run_copy(&cfg_path)?;
    }
    let cfg = Config::load_from_path(&cfg_path)
        .with_context(|| format!("loading config at {}", cfg_path.display()))?;

    if cli.validate {
        println!("OK: config at {} is valid.", cfg_path.display());
        return Ok(());
    }

    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let s = shutdown.clone();
        ctrlc::set_handler(move || s.store(true, Ordering::SeqCst))
            .context("installing Ctrl-C handler")?;
    }

    app::run(cfg, app::RunOptions { dry_run: cli.dry_run, shutdown })
}

fn resolve_config_path(explicit: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(p) = explicit { return Ok(p.to_path_buf()); }
    let base = dirs::config_dir()
        .context("could not determine user config directory")?;
    Ok(base.join("dualsense-mapper").join("config.json"))
}

fn first_run_copy(target: &std::path::Path) -> Result<()> {
    let example_text = include_str!("../config.example.json");
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating config dir {}", dir.display()))?;
    }
    std::fs::write(target, example_text)
        .with_context(|| format!("writing default config to {}", target.display()))?;
    eprintln!("[first run] wrote default config: {}", target.display());
    eprintln!("[first run] edit this file to customize your mapping, then re-run.");
    // Refuse to start so the user actually sees the file path.
    anyhow::bail!("first-run config written; re-run after reviewing it");
}

fn list_buttons(cfg: Option<Config>) -> Result<()> {
    let label_for = |id: u32| -> String {
        cfg.as_ref()
            .and_then(|c| c.buttons.get(&id.to_string()))
            .map(|e| e.label.clone())
            .unwrap_or_else(|| "<no label — id not in config>".into())
    };

    let mut source = GamepadSource::new()?;
    println!("Press a button or move a stick. Ctrl-C to quit.");

    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let s = shutdown.clone();
        ctrlc::set_handler(move || s.store(true, Ordering::SeqCst))?;
    }

    let mut buf = Vec::new();
    while !shutdown.load(Ordering::SeqCst) {
        buf.clear();
        source.poll(&mut buf);
        for ev in buf.drain(..) {
            match ev {
                GamepadEvent::Connected    => println!("🎮 Connected"),
                GamepadEvent::Disconnected => println!("🎮 Disconnected"),
                GamepadEvent::ButtonDown(id) =>
                    println!("  [button] id={id:<3} down       label: {:?}", label_for(id)),
                GamepadEvent::ButtonUp(id) =>
                    println!("  [button] id={id:<3} up         label: {:?}", label_for(id)),
                GamepadEvent::Stick { axis, value } =>
                    println!("  [axis  ] id={axis:<3} v={value:+.2}     (stick)"),
                GamepadEvent::Trigger { axis, value } =>
                    println!("  [trig  ] id={axis:<3} v={value:+.2}     (trigger normalized)"),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(())
}
