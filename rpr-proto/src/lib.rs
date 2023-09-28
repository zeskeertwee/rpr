pub enum ClientMessage {
    RequestConnection {
        application_id: [u8; 4],
    },
    InitializeConnection {
        challenge_response: [u8; 64]
    },
    // Starts the data stream
    SubmitReport {
        report_size: u32,
        report_hash: u32, // CRC32 hash
    },
}

pub enum ServerMessage {
    Challenge {
        data: [u8; 512],
    },
    ConnectionInitialized {
        version: u8,
        size_limit: u32,
    },
    // Closes the connection
    ReportReceived {
        report_id: [u8; 16]
    }
}