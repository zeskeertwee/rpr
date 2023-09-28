use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use log::{info, trace};

#[derive(Deserialize)]
pub struct Application {
    name: String,
    id: [u8; 4],
    key: String,
}

pub fn load_applications() -> Vec<Application> {
    let path = match std::env::var("APPLICATIONS_FOLDER") {
        Ok(v) => v,
        Err(_) => {
            let mut path = std::env::current_dir().unwrap().to_str().unwrap().to_string();
            path.push_str("/applications");
            info!("Using default application directory: {}", path);
            path.to_string()
        }
    };
    let path: PathBuf = PathBuf::from(path).canonicalize().unwrap();

    trace!("Reading application definitions from '{}'", path.to_string_lossy());

    //fs::read_dir(path)
    vec![]
}