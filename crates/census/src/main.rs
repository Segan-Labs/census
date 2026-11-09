use std::io::IsTerminal;
use std::process::ExitCode;

use census_collectors::SnapshotBuilder;
use census_core::{Config, Snapshot};
use census_layout::terminal_size;
use census_protocol::get_snapshot;
use census_render::{render_to_string, Options};

const HELP: &str = "census: system information from a daemon that already collected it

usage: census [options]

  --layout NAME     classic (default), compact or dashboard
  --logo NAME       auto (default), eclipse, neutral or none
  --width N         lay out for N columns instead of the terminal's
  --height N        lay out for N rows instead of the terminal's
  --color WHEN      auto (default), always or never
  --json            print the snapshot as JSON instead of rendering it
  --snapshot FILE   render a snapshot saved with --json
  -h, --help        this text
  -V, --version     the version

censusd keeps a snapshot warm; without it census collects directly, which is slower.";

struct Args {
    opts: Options,
    json: bool,
    snapshot_file: Option<String>,
    color: String,
    width: Option<usize>,
    height: Option<usize>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut a = Args {
        opts: Options::default(),
        json: false,
        snapshot_file: None,
        color: "auto".into(),
        width: None,
        height: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{} needs a value", name));
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", HELP);
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("census {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--layout" => a.opts.style = Some(value("--layout")?),
            "--logo" => {
                let v = value("--logo")?;
                if !["auto", "eclipse", "neutral", "none"].contains(&v.as_str()) {
                    return Err(format!("unknown logo '{}' (use auto, eclipse, neutral, none)", v));
                }
                a.opts.logo = Some(v);
            }
            "--width" => a.width = Some(value("--width")?.parse().map_err(|_| "--width needs a number")?),
            "--height" => a.height = Some(value("--height")?.parse().map_err(|_| "--height needs a number")?),
            "--color" => {
                a.color = value("--color")?;
                if !["auto", "always", "never"].contains(&a.color.as_str()) {
                    return Err("--color takes auto, always or never".into());
                }
            }
            "--json" => a.json = true,
            "--snapshot" => a.snapshot_file = Some(value("--snapshot")?),
            other => return Err(format!("unknown option '{}' (try --help)", other)),
        }
    }
    Ok(Some(a))
}

fn main() -> ExitCode {
    let mut args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("census: {}", e);
            return ExitCode::from(2);
        }
    };
    match run(&mut args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("census: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(args: &mut Args) -> Result<(), String> {
    let config = Config::load();
    let snapshot = load_snapshot(args, &config)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?);
        return Ok(());
    }

    let tty = std::io::stdout().is_terminal();
    let size = terminal_size();
    args.opts.width = args.width.or(size.map(|s| s.0)).unwrap_or(80);
    args.opts.height = args.height.or(size.map(|s| s.1));
    args.opts.color = match args.color.as_str() {
        "always" => true,
        "never" => false,
        _ => tty && std::env::var_os("NO_COLOR").is_none(),
    };
    let text = render_to_string(&snapshot, &config, &args.opts)?;
    use std::io::Write;
    let _ = std::io::stdout().write_all(text.as_bytes());
    Ok(())
}

fn load_snapshot(args: &Args, config: &Config) -> Result<Snapshot, String> {
    if let Some(file) = &args.snapshot_file {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{}: {}", file, e))?;
        return serde_json::from_str(&text).map_err(|e| format!("{}: {}", file, e));
    }
    if let Ok(snapshot) = get_snapshot(&config.daemon.socket_path) {
        return Ok(snapshot);
    }
    eprintln!("censusd is not running, collecting directly (slower). Start it: systemctl --user enable --now censusd");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(rt.block_on(SnapshotBuilder::new(config).build()))
}
