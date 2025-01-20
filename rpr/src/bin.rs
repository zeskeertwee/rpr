use rpr::initialize;

fn main() {
    let config = rpr::Configuration {
        interactive: true,
        use_fallback: true,
        fallback_address: "[YOUR IP/URL]".to_string(),
        address: "[YOUR IP/URL]".to_string(),
        shared_key: "[YOUR KEY]".to_string(), // generate with `openssl rand -base64 64`
        app_id: [84, 69, 83, 84, 0, 0],
    };
    initialize(config);
    panic!("test panic");
}