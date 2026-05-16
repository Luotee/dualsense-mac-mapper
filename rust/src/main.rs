use anyhow::{Context, Result};
use clap::{ArgAction, Parser};
use dualsense_mapper::app;
use dualsense_mapper::config::Config;
use dualsense_mapper::gamepad::{GamepadEvent, GamepadSource};
use dualsense_mapper::safety;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "dualsense-mapper", version, about)]
struct Cli {
    /// Path to config.json. Defaults to <exe_dir>/dualsense-mapper.json.
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

    /// Do not pause for "Press Enter to close" on error (CLI scripts / CI).
    #[arg(long, action = ArgAction::SetTrue)]
    no_pause: bool,

    /// Run in legacy console mode (v0.1.x behaviour). Default is GUI.
    #[arg(long, action = ArgAction::SetTrue)]
    cli: bool,
}

fn main() {
    // We parse once up-front to know whether to pause on error. If parsing
    // itself fails, clap prints its own message and exits cleanly — pause
    // afterward so a double-click user can read what was wrong.
    let cli_parse = Cli::try_parse();
    let no_pause = match &cli_parse {
        Ok(c) => c.no_pause,
        Err(_) => false,
    };
    let cli = match cli_parse {
        Ok(c) => c,
        Err(e) => {
            // clap renders --help / --version through this same path.
            // Pause only on actual error kinds.
            let kind = e.kind();
            let _ = e.print();
            if matches!(
                kind,
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                std::process::exit(0);
            }
            if !no_pause { pause_on_error(); }
            std::process::exit(2);
        }
    };

    if let Err(e) = real_main(&cli) {
        eprintln!();
        eprintln!("Error: {e:#}");
        eprintln!();
        if !cli.no_pause {
            pause_on_error();
        }
        std::process::exit(1);
    }
}

fn real_main(cli: &Cli) -> Result<()> {
    let filter = if cli.verbose {
        EnvFilter::new("dualsense_mapper=debug,info")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Iron rule #3: panic on any thread must release held keys at the OS level.
    // Installed here — before CLI or GUI dispatch — so both paths are covered
    // from a single site. The Drop on KeyboardSink covers the normal shutdown
    // path; this hook covers the abnormal (panic) path.
    let panic_default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Err(e) = safety::emergency_release_all() {
            eprintln!("emergency key release failed: {e:#}");
        }
        panic_default(info);
    }));

    let cfg_path = resolve_config_path(cli.config.as_deref())?;

    if cli.list_buttons {
        // For list-buttons, missing config is OK — show "<no label>".
        let cfg = Config::load_from_path(&cfg_path).ok();
        return list_buttons(cfg);
    }

    // Every other path needs a valid config; if it doesn't exist, write the
    // bundled default beside the exe. Unlike v0.1.0, we keep running with
    // that default — a double-clicked exe should "just work" out of the box.
    let just_wrote_default = if !cfg_path.exists() {
        first_run_copy(&cfg_path)?;
        true
    } else {
        false
    };
    let cfg = Config::load_from_path(&cfg_path)
        .with_context(|| format!("loading config at {}", cfg_path.display()))?;

    if cli.validate {
        println!("OK: config at {} is valid.", cfg_path.display());
        return Ok(());
    }

    // --cli: legacy console mode (v0.1.x behaviour). Opt-in from v0.2.0.
    if cli.cli {
        print_banner(&cfg_path, cli.dry_run, just_wrote_default);
        let shutdown = Arc::new(AtomicBool::new(false));
        {
            let s = shutdown.clone();
            ctrlc::set_handler(move || s.store(true, Ordering::SeqCst))
                .context("installing Ctrl-C handler")?;
        }
        return app::run(cfg, app::RunOptions { dry_run: cli.dry_run, shutdown });
    }

    // Default: GUI mode (v0.2.0+).
    #[cfg(feature = "gui")]
    {
        return dualsense_mapper::gui::run(
            cfg,
            dualsense_mapper::gui::RunOptions {
                config_path: cfg_path,
                dry_run: cli.dry_run,
            },
        );
    }

    // Reached only when compiled without --features gui.
    #[cfg(not(feature = "gui"))]
    {
        anyhow::bail!(
            "This binary was built without GUI support. Re-run with --cli, \
             or rebuild with `cargo build --features gui`."
        );
    }
}

fn print_banner(cfg_path: &std::path::Path, dry_run: bool, just_wrote_default: bool) {
    println!("=================================================");
    println!(" DualSense Mapper v{}", env!("CARGO_PKG_VERSION"));
    println!(" Config: {}", cfg_path.display());
    if dry_run {
        println!(" Mode:   DRY-RUN (no real keystrokes sent)");
    }
    println!("-------------------------------------------------");
    if just_wrote_default {
        println!(" Wrote default config — open the file above in");
        println!(" Notepad to customize. The file has an inline");
        println!(" keyboard cheat sheet at the top.");
        println!(" Restart this program after you save changes.");
        println!("-------------------------------------------------");
    }
    println!(" Plug in DualSense via USB. Press buttons; the");
    println!(" mapped keys are sent to the focused window.");
    println!(" Press Ctrl-C or close this window to quit.");
    println!("=================================================");
    println!();
}

fn pause_on_error() {
    eprintln!("Press Enter to close this window.");
    let _ = std::io::stderr().flush();
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}

fn resolve_config_path(explicit: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(p) = explicit { return Ok(p.to_path_buf()); }
    // Portable layout: config sits next to the executable as dualsense-mapper.json.
    // Drop the folder, move it elsewhere, the config follows.
    let exe = std::env::current_exe().context("could not determine executable path")?;
    let dir = exe.parent().context("executable has no parent directory")?;
    Ok(dir.join("dualsense-mapper.json"))
}

fn first_run_copy(target: &std::path::Path) -> Result<()> {
    let example_text = include_str!("../config.example.json");
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating config dir {}", dir.display()))?;
    }
    std::fs::write(target, example_text)
        .with_context(|| format!("writing default config to {}", target.display()))?;
    // Don't bail — the bundled default is valid, so we continue with it.
    // The startup banner will tell the user the file was created.
    Ok(())
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
