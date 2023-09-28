use crate::application::load_applications;

mod application;

fn main() {
    pretty_env_logger::init();
    let applications = load_applications();
    println!("Hello, world!");
}
