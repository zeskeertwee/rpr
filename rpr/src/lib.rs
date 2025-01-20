#![feature(ascii_char)]

use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::panic::PanicInfo;
use std::sync::Arc;
use text_io::read;
use uuid::Uuid;
use rpr_proto::{ClientMessage, ServerMessage};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const SERVER_VERSION: u8 = 1;
const HELP: &'static str = r#"Commands:
 y  - Submit crash report
 n  - Do not submit crash report
 v  - View crash report
 h  - Help
ver - Version info
cfg - Configuration info
"#;

#[derive(Clone)]
pub struct Configuration {
    pub address: String,
    pub use_fallback: bool,
    // Used if the address failed to connect (i.e. because it's an external IP) and fallback is enabled, fallback can be used for internal IP when testing
    pub fallback_address: String,
    pub app_id: [u8; 6],
    pub shared_key: String,
    // Set to false to automatically submit on panic (i.e. daemons), true to ask the user for permission
    pub interactive: bool
}

pub fn initialize(cfg: Configuration) {
    std::panic::set_hook(Box::new(move |info| {
        match panic_handler(info, &cfg) {
            Ok(_) => (),
            Err(e) => println!("Fatal error occured during crash report submission: {}", e),
        }
    }))
}

fn panic_handler(info: &PanicInfo, cfg: &Configuration) -> anyhow::Result<()> {
    let report = rpr_proto::generate_report(info);

    if cfg.interactive {
        println!("Oops! It seems the application has crashed!");
        println!("Would you like to submit a crash report?");
        println!("(type 'y' to submit, 'n' to not submit, 'v' to view the crash report, and 'h' for help)");
    } else {
        println!("The application has crashed, automatically submitting crash report (non-interactive mode).")
    }

    // loop only when interactive
    while cfg.interactive {
        print!("crash-reporter > ");
        let cmd: String = read!("{}\n");

        match cmd.to_lowercase().as_str() {
            "n" | "q" | "quit" | "exit" => {
                println!("Exiting...");
                std::process::exit(-1);
            },
            "h" | "help" => {
                println!("{}", HELP);
            }
            "cfg" => {
                println!("Configuration:");
                println!("Server address: {}", cfg.address);
                println!("Fallback address: {} [In use: {}]", cfg.fallback_address, cfg.use_fallback);
                println!("Application ID: {}", cfg.app_id.as_ascii().map(|v| v.into_iter().map(|c| c.to_char()).collect::<String>()).unwrap_or(format!("{:?}", cfg.app_id)));
            }
            "v" => {
                println!(" --- CRASH REPORT ---");
                println!("{}", report);
            }
            "ver" => {
                println!("crash-reporter shell v{}", VERSION);
            }
            "y" => {
                break;
            }
            other => {
                println!("'{}' is not a valid command", other);
            }
        }
    }

    print!("Connecting to crash report server...  ");
    std::io::stdout().flush(); // make sure we print the above to the terminal
    let mut stream = match TcpStream::connect(&cfg.address) {
        Ok(s) => s,
        Err(_) => {
            // fall back to local IP
            if cfg.use_fallback {
                println!("Falling back to {}", cfg.fallback_address);
            } else {
                anyhow::bail!("Failed to connect to panic-report server!")
            }

            match TcpStream::connect(&cfg.fallback_address) {
                Ok(s) => s,
                Err(_) => anyhow::bail!("Failed to connect to panic-report fallback server!"),
            }
        },
    };
    println!("Connected!");
    std::io::stdout().flush();
    rpr_proto::send_message(&mut stream, ClientMessage::RequestConnection {
        application_id: cfg.app_id
    })?;
    print!("Authorizing... ");

    let solution = match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::Challenge { data } => rpr_proto::solve_challenge(data, &cfg.shared_key)?,
        _ => anyhow::bail!("Unexpected message!")
    };

    rpr_proto::send_message(&mut stream, ClientMessage::InitializeConnection {
        challenge_response: solution
    })?;
    let limit = match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::ConnectionInitialized { size_limit, version } => {
            //println!("Server accepted connection, server version {}, size limit {}KiB", version, size_limit / 1024);
            if version != SERVER_VERSION {
                anyhow::bail!("Server version mismatch!");
            }
            size_limit
        },
        _ => anyhow::bail!("Unexpected message!")
    };
    println!("Accepted");

    let report_bin = report.as_bytes().to_vec();
    if report_bin.len() as u32 > limit {
        println!("Report is bigger than server's size limit!");
        println!("Unable to submit report!");
        anyhow::bail!("Report too big!");
    }

    print!("Announcing crash report... ");
    std::io::stdout().flush();
    rpr_proto::send_message(&mut stream, ClientMessage::SubmitReport {
        report_hash: rpr_proto::compute_hash(&report_bin),
        report_size: report_bin.len() as u32,
    })?;
    println!("done");

    print!("Starting crash report data stream... ");
    std::io::stdout().flush();
    stream.write_all(&report_bin)?;
    println!("report sent");

    match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::ReportReceived { report_id } => println!("Crash report received, report ID {}", Uuid::from_u128(report_id)),
        _ => anyhow::bail!("Unexpected message!")
    };

    stream.shutdown(Shutdown::Both)?;

    if cfg.interactive {
        println!("Thank you for submitting the crash report!");
    }

    std::process::exit(-1);
}
