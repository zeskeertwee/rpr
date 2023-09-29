use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;
use toml;
use wherr::wherr;
use log::{error, info, trace};
use anyhow::Result;

#[derive(Deserialize)]
pub struct Application {
    pub name: String,
    pub id: [u8; 6],
    pub key: String,
}

#[wherr]
pub fn load_applications() -> Result<HashMap<[u8; 6], Application>> {
    let path = match std::env::var("APPLICATIONS_FOLDER") {
        Ok(v) => v,
        Err(_) => {
            let mut path = std::env::current_dir()?.to_string_lossy().to_string();
            path.push_str("/applications");
            info!("Using default application directory: {}", path);
            path.to_string()
        }
    };
    let path: PathBuf = PathBuf::from(path).canonicalize()?;

    trace!("Reading application definitions from '{}'", path.to_string_lossy());

    let mut apps: HashMap<[u8; 6], Application> = HashMap::new();
    for i in fs::read_dir(path).unwrap() {
        match i {
            Ok(v) => {
                trace!("Reading '{}'", v.path().to_string_lossy());
                let data = fs::read_to_string(v.path())?;
                let table: toml::Table = toml::from_str(&data)?;

                for i in table {
                    let appdef: Application = i.1.try_into()?;
                    trace!("Loaded application definition '{}'", appdef.name);
                    apps.insert(appdef.id, appdef);
                }
            },
            Err(e) => {
                error!("Unable to open file: {}", e.to_string())
            }
        }
    }
    info!("Application definition loading finished, {} appdefs loaded", apps.len());

    Ok(apps)
}