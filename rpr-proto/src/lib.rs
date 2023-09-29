use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;
use anyhow::Result;
use serde::de::DeserializeOwned;
use sha3::Sha3_512;
use hmac::{Hmac, Mac, digest::FixedOutput};
use base64::{Engine, engine::general_purpose};
use log::trace;

mod report;
pub use report::generate_report;

type HmacSha512 = Hmac<Sha3_512>;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    RequestConnection {
        application_id: [u8; 6],
    },
    InitializeConnection {
        #[serde(with = "BigArray")]
        challenge_response: [u8; 64]
    },
    // Starts the data stream
    SubmitReport {
        report_size: u32,
        report_hash: u32, // CRC32 hash
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    Challenge {
        #[serde(with = "BigArray")]
        data: [u8; 512],
    },
    ConnectionInitialized {
        version: u8,
        size_limit: u32,
    },
    // Closes the connection
    ReportReceived {
        report_id: u128
    }
}

pub fn send_message<W: Write, S: Serialize>(writer: &mut W, message: S) -> Result<()> {
    let message_bin = bincode::serialize(&message)?;

    writer.write_all(&(message_bin.len() as u32).to_le_bytes())?;
    writer.write_all(&message_bin)?;
    Ok(())
}

pub fn receive_message<R: Read, S: DeserializeOwned>(reader: &mut R) -> Result<S> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    let length = u32::from_le_bytes(buf) as usize;
    let mut dbuf = vec![0; length];
    reader.read_exact(&mut dbuf)?;
    Ok(bincode::deserialize(&dbuf)?)
}

pub fn solve_challenge(data: [u8; 512], key: &str) -> Result<[u8; 64]> {
    let key_data = general_purpose::STANDARD.decode(key)?;
    trace!("Decoded {}-byte key", key_data.len());

    let mut mac = HmacSha512::new_from_slice(&key_data)?;
    mac.update(&data);

    let result = mac.finalize_fixed();
    let mut awnser = [0; 64];
    awnser.copy_from_slice(result.as_slice());
    
    Ok(awnser)
}

pub fn compute_hash(data: &Vec<u8>) -> u32 {
    crc32fast::hash(data)
}