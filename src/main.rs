use std::{
    error::Error,
    ffi::CString,
    fs,
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use clap::{Args, Parser, Subcommand};
use feightstick::{
    device::{Panel, Usb},
    mapping::{DEFAULT_PROFILE, GamepadState, Mapper, Profile},
    output::{Gamepad, Output},
    protocol::{Decoder, Frame, REPORT_LEN, Side},
};

#[derive(Parser)]
#[command(
    version,
    about = "FEI control panels as native input and a virtual gamepad"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the platform's virtual gamepad bridge (also the default command).
    Run {
        #[command(flatten)]
        input: InputOptions,
        /// Optional TOML profile. Otherwise the built-in default is used.
        #[arg(long)]
        profile: Option<PathBuf>,
        /// Stop cleanly when this file is created (used by the Windows installer).
        #[arg(long)]
        shutdown_file: Option<PathBuf>,
    },
    /// Stream decoded reports as JSON lines, without creating a gamepad.
    Monitor {
        #[command(flatten)]
        input: InputOptions,
    },
    /// List connected FEI panel paths.
    Devices,
    /// Decode stdin JSON lines containing arrays of exactly 20 byte values.
    Decode {
        #[arg(long, value_enum)]
        side: Side,
        /// Declare the joystick centered before the first report.
        #[arg(long)]
        centered: bool,
    },
    /// Print the built-in TOML gamepad profile.
    Profile,
    /// Validate a TOML profile without opening hardware.
    CheckProfile { path: PathBuf },
}

#[derive(Args, Default)]
struct InputOptions {
    /// Select a specific left device when several are attached.
    #[arg(long)]
    left_path: Option<String>,
    /// Select a specific right device when several are attached.
    #[arg(long)]
    right_path: Option<String>,
    /// Assume the joystick is physically centered at every connection.
    #[arg(long)]
    center_on_connect: bool,
}

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

fn main() -> std::process::ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // Closing a pipe (e.g. monitor | head) is an ordinary exit.
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
            {
                return std::process::ExitCode::SUCCESS;
            }
            eprintln!("feightstick: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result {
    let command = cli.command.unwrap_or(Command::Run {
        input: InputOptions::default(),
        profile: None,
        shutdown_file: None,
    });
    match command {
        Command::Devices => {
            for info in Usb::new()?.refresh()? {
                println!(
                    "{}\t{}\t{}",
                    info.side,
                    info.name,
                    info.path.to_string_lossy()
                );
            }
            Ok(())
        }
        Command::Profile => {
            io::stdout().write_all(DEFAULT_PROFILE.as_bytes())?;
            Ok(())
        }
        Command::CheckProfile { path } => {
            let profile = Profile::parse(&fs::read_to_string(path)?)?;
            println!("Valid profile: {} bindings", profile.bindings.len());
            Ok(())
        }
        Command::Decode { side, centered } => {
            let mut decoder = Decoder::new(side);
            if centered {
                decoder.center();
            }
            let mut stdout = io::stdout().lock();
            for (line_number, line) in io::stdin().lock().lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let bytes: Vec<u8> = serde_json::from_str(&line).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("line {}: {error}", line_number + 1),
                    )
                })?;
                write_frame(&mut stdout, &decoder.decode(&bytes)?)?;
            }
            Ok(())
        }
        Command::Monitor { input } => listen(input, None, None),
        Command::Run {
            input,
            profile,
            shutdown_file,
        } => {
            let text = profile.map(fs::read_to_string).transpose()?;
            let profile = Profile::parse(text.as_deref().unwrap_or(DEFAULT_PROFILE))?;
            listen(input, Some(profile), shutdown_file)
        }
    }
}

fn write_frame(writer: &mut impl Write, frame: &Frame) -> Result {
    serde_json::to_writer(&mut *writer, frame)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

struct Connection {
    panel: Panel,
    decoder: Decoder,
}

enum UserCommand {
    Center,
    Quit,
}

struct Bridge {
    output: Gamepad,
    mapper: Mapper,
    previous: Option<GamepadState>,
}

impl Bridge {
    fn new(profile: Profile) -> Result<Self> {
        Ok(Self {
            output: Gamepad::new()?,
            mapper: Mapper::new(profile),
            previous: None,
        })
    }

    fn update(&mut self, frame: &Frame, now: Instant) -> Result {
        self.mapper.update(frame, now);
        self.flush(now)
    }

    fn disconnect(&mut self, side: Side) -> Result {
        self.mapper.disconnect(side);
        self.flush(Instant::now())
    }

    fn flush(&mut self, now: Instant) -> Result {
        let state = self.mapper.state(now);
        if self.previous != Some(state) {
            self.output.send(&state)?;
            self.previous = Some(state);
        }
        Ok(())
    }
}

fn listen(
    options: InputOptions,
    profile: Option<Profile>,
    shutdown_file: Option<PathBuf>,
) -> Result {
    let mut usb = Usb::new()?;
    let mut bridge = profile.map(Bridge::new).transpose()?;
    let running = Arc::new(AtomicBool::new(true));
    let signal_running = Arc::clone(&running);
    ctrlc::set_handler(move || signal_running.store(false, Ordering::Relaxed))?;
    let (commands, receiver) = mpsc::channel();
    if io::stdin().is_terminal() {
        eprintln!(
            "Waiting for FEI panels. Enter c to center the joystick, q to quit; Ctrl-C also stops."
        );
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                let Ok(line) = line else {
                    break;
                };
                let command = match line.trim() {
                    "c" => UserCommand::Center,
                    "q" => UserCommand::Quit,
                    _ => continue,
                };
                if commands.send(command).is_err() {
                    break;
                }
            }
        });
    } else if options.center_on_connect {
        eprintln!("Waiting for FEI panels. Keep the joystick released when connecting.");
    } else {
        eprintln!(
            "Waiting for FEI panels. Joystick needs --center-on-connect in noninteractive use."
        );
    }
    let paths = [
        options.left_path.map(CString::new).transpose()?,
        options.right_path.map(CString::new).transpose()?,
    ];
    let mut connections: [Option<Connection>; 2] = [None, None];
    let mut next_scan = Instant::now();
    let mut stdout = io::stdout().lock();
    while running.load(Ordering::Relaxed) {
        if let Some(path) = &shutdown_file {
            if path.try_exists()? {
                break;
            }
        }
        while let Ok(command) = receiver.try_recv() {
            match command {
                UserCommand::Quit => running.store(false, Ordering::Relaxed),
                UserCommand::Center => {
                    let Some(connection) = &mut connections[Side::Right.index()] else {
                        eprintln!("Connect the right panel before centering.");
                        continue;
                    };
                    // Consume queued reports before resetting the origin, retaining
                    // button releases even when no later packet arrives.
                    let mut idle = false;
                    for _ in 0..64 {
                        let mut buffer = [0u8; REPORT_LEN + 1];
                        let Some(length) = connection.panel.read(&mut buffer)? else {
                            idle = true;
                            break;
                        };
                        let frame = connection.decoder.decode(&buffer[..length])?;
                        if let Some(bridge) = &mut bridge {
                            bridge.update(&frame, Instant::now())?;
                        } else {
                            write_frame(&mut stdout, &frame)?;
                        }
                    }
                    if idle {
                        connection.decoder.center();
                        if let Some(bridge) = &mut bridge {
                            bridge.mapper.center();
                            bridge.flush(Instant::now())?;
                        }
                        eprintln!("Joystick origin set to the current physical position.");
                    } else {
                        eprintln!("Joystick is busy; let it rest at center and enter c again.");
                    }
                }
            }
        }
        let now = Instant::now();
        if now >= next_scan {
            let devices = usb.refresh()?;
            for side in [Side::Left, Side::Right] {
                let slot = &mut connections[side.index()];
                if slot.as_ref().is_some_and(|connection| {
                    !devices
                        .iter()
                        .any(|info| info.path == connection.panel.info.path)
                }) {
                    *slot = None;
                    if let Some(bridge) = &mut bridge {
                        bridge.disconnect(side)?;
                    }
                    eprintln!("{side}: disconnected");
                }
                if slot.is_none() {
                    match usb.open(&devices, side, paths[side.index()].as_deref()) {
                        Ok(Some(panel)) => {
                            let mut decoder = Decoder::new(side);
                            if options.center_on_connect {
                                decoder.center();
                            }
                            eprintln!("{side}: connected");
                            *slot = Some(Connection { panel, decoder });
                        }
                        Ok(None) => (),
                        Err(error) => eprintln!("{side}: {error}"),
                    }
                }
            }
            next_scan = now + Duration::from_secs(1);
        }
        for side in [Side::Left, Side::Right] {
            let slot = &mut connections[side.index()];
            let Some(connection) = slot else {
                continue;
            };
            // Bound each drain so a busy panel cannot starve the other panel or shutdown.
            for _ in 0..64 {
                let mut buffer = [0u8; REPORT_LEN + 1];
                match connection.panel.read(&mut buffer) {
                    Ok(Some(length)) => match connection.decoder.decode(&buffer[..length]) {
                        Ok(frame) => {
                            if frame.sequence_gap {
                                if side == Side::Right {
                                    eprintln!(
                                        "{side}: report sequence gap; re-center the joystick with c"
                                    );
                                } else {
                                    eprintln!("{side}: report sequence gap");
                                }
                            }
                            if let Some(bridge) = &mut bridge {
                                bridge.update(&frame, now)?;
                            } else {
                                write_frame(&mut stdout, &frame)?;
                            }
                        }
                        Err(error) => {
                            // Malformed input invalidates retained output until a valid report.
                            if let Some(bridge) = &mut bridge {
                                bridge.disconnect(side)?;
                            }
                            connection.decoder = Decoder::new(side);
                            eprintln!("{side}: {error}");
                        }
                    },
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!("{side}: disconnected: {error}");
                        *slot = None;
                        if let Some(bridge) = &mut bridge {
                            bridge.disconnect(side)?;
                        }
                        break;
                    }
                }
            }
        }
        if let Some(bridge) = &mut bridge {
            bridge.flush(Instant::now())?;
        }
        thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}
