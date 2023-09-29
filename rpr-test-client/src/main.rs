use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::panic::PanicInfo;
use anyhow::Result;
use uuid::Uuid;
use rpr_proto;
use rpr_proto::{ClientMessage, ServerMessage};

const KEY: &'static str = "ZfAr2p3QdzAasrBNkNH540kGbxu62KTF5uSerJGfx/tZ2P6vqK6HJFYkMxL77lkeFfPfY7Fk+sNgtoCSNtFUwQ==";

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(move |info| {
        submit_backtrace(info).unwrap();
    }));

    panic!("uh oh");
}

fn submit_backtrace(info: &PanicInfo) -> Result<()> {
    println!("Connecting to 127.0.0.1:9001");
    let mut stream = TcpStream::connect("127.0.0.1:9001")?;

    rpr_proto::send_message(&mut stream, ClientMessage::RequestConnection {
        application_id: [41, 54, 52, 41, 50, 49]
    })?;
    println!("Sent connection request");

    let solution = match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::Challenge { data } => {
            rpr_proto::solve_challenge(data, KEY)?
        },
        _ => {
            print!("Unexpected message!");
            return Ok(());
        }
    };
    println!("Got challenge");

    rpr_proto::send_message(&mut stream, ClientMessage::InitializeConnection {
        challenge_response: solution
    })?;
    println!("Submitted challenge solution");

    let limit = match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::ConnectionInitialized { size_limit, version } => {
            println!("Server accepted connection, server version {}, size limit {}KiB", version, size_limit / 1024);
            size_limit
        },
        _ => {
            print!("Unexpected message!");
            return Ok(());
        }
    };

    let report = rpr_proto::generate_report(info);
    let report_bin = report.as_bytes().to_vec();
    rpr_proto::send_message(&mut stream, ClientMessage::SubmitReport {
        report_hash: rpr_proto::compute_hash(&report_bin),
        report_size: report_bin.len() as u32,
    })?;
    println!("Starting data stream, size {}KiB, CRC32 {}", report_bin.len() / 1024, rpr_proto::compute_hash(&report_bin));
    stream.write_all(&report_bin)?;

    match rpr_proto::receive_message(&mut stream)? {
        ServerMessage::ReportReceived { report_id } => {
            println!("Server received report, ID {}", Uuid::from_u128(report_id));
        },
        _ => {
            print!("Unexpected message!");
            return Ok(());
        }
    };
    stream.shutdown(Shutdown::Both)?;


    Ok(())
}
