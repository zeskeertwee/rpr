use std::collections::HashMap;
use std::io::{Read, Write};
use crate::application::{Application, load_applications};
use std::net::{Shutdown, TcpListener, TcpStream};
use anyhow::Result;
use log::{error, info, trace};
use rand::RngCore;
use wherr::wherr;
use uuid::Uuid;
use rpr_proto::{ClientMessage, ServerMessage};
use std::fs::File;

pub mod application;

const VERSION: u8 = 1;

#[wherr]
fn main() -> Result<()> {
    pretty_env_logger::init();
    let applications = load_applications()?;
    let report_path = match std::env::var("REPORT_DIR") {
        Ok(v) => v.to_string(),
        Err(_) => {
            let mut dir = std::env::current_dir()?;
            dir.push("reports");
            dir = dir.canonicalize()?;
            info!("Using default report folder '{}'", dir.to_string_lossy());
            dir.to_string_lossy().to_string()
        }
    };

    info!("Binding TCP listener to 0.0.0.0:9001");
    let listener = TcpListener::bind("0.0.0.0:9001")?;
    for i in listener.incoming() {
        match handle_connection(i?, &applications, &report_path) {
            Ok(_) => (),
            Err(e) => log::warn!("Connection handling failed with error: {}", e),
        }
    }

    Ok(())
}

#[wherr]
fn handle_connection(mut stream: TcpStream, appdefs: &HashMap<[u8; 6], Application>, report_path: &str) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    trace!("Received connection from addr {}", peer_addr);

    let app = match rpr_proto::receive_message(&mut stream)? {
        ClientMessage::RequestConnection { application_id } => {
            let app = match appdefs.get(&application_id) {
                Some(v) => v,
                None => {
                    error!("Invalid application id from {}, ID {:?}, terminating connection", peer_addr, application_id);
                    stream.shutdown(Shutdown::Both)?;
                    return Ok(());
                }
            };
            trace!("Received connection request from {}, application '{}' appID {:?}", peer_addr, app.name, application_id);
            app
        },
        _ => {
            error!("Unexpected message from {}, terminating connection", peer_addr);
            stream.shutdown(Shutdown::Both)?;
            return Ok(());
        }
    };

    let mut challenge_data = [0; 512];
    rand::thread_rng().fill_bytes(&mut challenge_data);

    rpr_proto::send_message(&mut stream, ServerMessage::Challenge {
        data: challenge_data,
    })?;
    trace!("Sent challenge to {}", peer_addr);

    let solution = rpr_proto::solve_challenge(challenge_data, &app.key)?;
    match rpr_proto::receive_message(&mut stream)? {
        ClientMessage::InitializeConnection {
            challenge_response
        } => {
            if challenge_response == solution {
                trace!("challenge solved by {}, proceeding", peer_addr);
            } else {
                error!("challenge failed by {}, terminating connection", peer_addr);
                stream.shutdown(Shutdown::Both)?;
                return Ok(());
            }
        },
        _ => {
            error!("Unexpected message from {}, terminating connection", peer_addr);
            stream.shutdown(Shutdown::Both)?;
            return Ok(());
        }
    }

    rpr_proto::send_message(&mut stream, ServerMessage::ConnectionInitialized {
        size_limit: 1024 * 64, // max 64KiB
        version: VERSION,
    })?;

    let report = match rpr_proto::receive_message(&mut stream)? {
        ClientMessage::SubmitReport {
            report_size,
            report_hash
        } => {
            trace!("Receiving {}KiB report from {}, CRC32 {}", report_size / 1024, peer_addr, report_hash);
            if report_size > 1024 * 64 {
                // too big
                error!("Report from {} too big, terminating connection", peer_addr);
                stream.shutdown(Shutdown::Both)?;
                return Ok(());
            }
            
            let mut buf = vec![0; report_size as usize];
            stream.read_exact(&mut buf)?;
            if !(rpr_proto::compute_hash(&buf) == report_hash) {
                error!("CRC32 does not match for report from {}, terminating connection", peer_addr);
                stream.shutdown(Shutdown::Both)?;
                return Ok(());
            }
            trace!("Report received successfully");
            buf
        },
        _ => {
            error!("Unexpected message from {}, terminating connection", peer_addr);
            stream.shutdown(Shutdown::Both)?;
            return Ok(());
        }
    };

    let uuid = Uuid::from_u64_pair(rand::thread_rng().next_u64(), rand::thread_rng().next_u64());
    trace!("Generated report ID {} for report from {}", uuid, peer_addr);

    rpr_proto::send_message(&mut stream, ServerMessage::ReportReceived {
        report_id: uuid.as_u128()
    })?;
    stream.shutdown(Shutdown::Both)?;
    let mut file = File::create(format!("{}/{}-{}.txt", report_path, app.name, uuid))?;
    file.write_all(&report)?;
    trace!("Successfully saved report {}-{}.txt", app.name, uuid);

    Ok(())
}