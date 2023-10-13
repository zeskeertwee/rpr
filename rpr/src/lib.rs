use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::panic::PanicInfo;
use text_io::read;
use uuid::Uuid;
use rpr_proto::{ClientMessage, ServerMessage};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const SERVER_VERSION: u8 = 1;

pub fn initialize<K: ToString>(app_id: [u8; 6], shared_key: &K) {
    let shared_key= shared_key.to_string();
    std::panic::set_hook(Box::new(move |info| {
        match panic_handler(info, app_id, shared_key.clone()) {
            Ok(_) => (),
            Err(e) => println!("Fatal error occured during crash report submission: {}", e),
        }
    }))
}

fn panic_handler(info: &PanicInfo, app_id: [u8; 6], shared_key: String) -> anyhow::Result<()> {
    let report = rpr_proto::generate_report(info);

    println!("Oops! It seems the application has crashed!");
    println!("Would you like to submit a crash report?");
    println!("(type 'y' to submit, 'n' to not submit, and 'v' to view the crash report)");

    loop {
        print!("crash-reporter > ");
        let cmd: String = read!("{}\n");

        match cmd.to_lowercase().as_str() {
            "n" | "q" | "quit" | "exit" => {
                println!("Exiting...");
                std::process::exit(-1);
            }
            "v" => {
                println!(" --- CRASH REPORT ---");
                println!("{}", report);
            }
            "ver" => {
                println!("crash-reporter shell v{}", VERSION);
            }
            "y" => {
                print!("Connecting to crash report server...  ");
                let mut stream = match TcpStream::connect("fortunecookie.duckdns.org:9001") {
                    Ok(s) => s,
                    Err(_) => {
                        // fall back to local IP
                        if cfg!(debug_assertions) {
                            println!("Falling back to 192.168.0.202")
                        }

                        match TcpStream::connect("192.168.0.202:9001") {
                            Ok(s) => s,
                            Err(_) => anyhow::bail!("Failed to connect to panic-report server!"),
                        }
                    },
                };
                println!("connected!");

                rpr_proto::send_message(&mut stream, ClientMessage::RequestConnection {
                    application_id: app_id
                })?;
                print!("Authorizing... ");

                let solution = match rpr_proto::receive_message(&mut stream)? {
                    ServerMessage::Challenge { data } => rpr_proto::solve_challenge(data, &shared_key)?,
                    _ => anyhow::bail!("Unexpected message!")
                };

                rpr_proto::send_message(&mut stream, ClientMessage::InitializeConnection {
                    challenge_response: solution
                })?;
                print!("submitted... ");

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
                print!("accepted");

                let report_bin = report.as_bytes().to_vec();
                if report_bin.len() as u32 > limit {
                    println!("Report is bigger than server's size limit!");
                    println!("Unable to submit report!");
                    anyhow::bail!("Report too big!");
                }

                print!("Announcing crash report... ");
                rpr_proto::send_message(&mut stream, ClientMessage::SubmitReport {
                    report_hash: rpr_proto::compute_hash(&report_bin),
                    report_size: report_bin.len() as u32,
                })?;
                println!("done");

                print!("Starting crash report data stream... ");
                stream.write_all(&report_bin)?;
                println!("report sent");

                match rpr_proto::receive_message(&mut stream)? {
                    ServerMessage::ReportReceived { report_id } => println!("Crash report received, report ID {}", Uuid::from_u128(report_id)),
                    _ => anyhow::bail!("Unexpected message!")
                };
                stream.shutdown(Shutdown::Both)?;
                println!("Thank you for submitting the crash report!");
                std::process::exit(-1);
            }
            other => {
                println!("'{}' is not a valid command", other);
            }
        }
    }
}
