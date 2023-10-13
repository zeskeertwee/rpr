use os_info;
use backtrace;
use std::fmt::Write;
use std::panic::PanicInfo;

const HEX_WIDTH: usize = std::mem::size_of::<usize>() + 2;
const NEXT_SYMBOL_PADDING: usize = HEX_WIDTH + 6;

// based on handle_dump in https://github.com/rust-cli/human-panic
pub fn generate_report(info: &PanicInfo) -> String {
    let mut report: Vec<String> = vec![];

    let osi = os_info::get();
    report.push(format!("OS: {}", osi.os_type()));
    report.push(format!("OS version: {}", osi.version()));
    report.push(format!("Architecture: {:?}", osi.architecture()));
    report.push(format!("Bitness: {:?}\n", osi.architecture()));

    match match (
        info.payload().downcast_ref::<&str>(),
        info.payload().downcast_ref::<String>(),
    ) {
        (Some(s), _) => Some(s.to_string()),
        (_, Some(s)) => Some(s.to_string()),
        (None, None) => None,
    } {
        Some(v) => report.push(format!("Message: {}", v)),
        None => report.push("Message: --unknown--".to_string()),
    }

    report.push("\n--- BACKTRACE ---".to_string());
    let backtrace = backtrace::Backtrace::new();
    for (idx, frame) in backtrace.frames().iter().enumerate() {
        let mut backtrace = String::new();
        let ip = frame.ip();

        let _ = write!(backtrace, "{idx:4}: {ip:HEX_WIDTH$?}");

        let symbols = frame.symbols();
        if symbols.is_empty() {
            let _ = write!(backtrace, " - <unresolved>");
            continue;
        }

        for (idx, symbol) in symbols.iter().enumerate() {
            //Print symbols from this address,
            //if there are several addresses
            //we need to put it on next line
            if idx != 0 {
                let _ = write!(backtrace, "\n{:1$}", "", NEXT_SYMBOL_PADDING);
            }

            if let Some(name) = symbol.name() {
                let _ = write!(backtrace, " - {name}");
            } else {
                let _ = write!(backtrace, " - <unknown>");
            }

            //See if there is debug information with file name and line
            if let (Some(file), Some(line)) = (symbol.filename(), symbol.lineno()) {
                let _ = write!(
                    backtrace,
                    "\n{:3$}at {}:{}",
                    "",
                    file.display(),
                    line,
                    NEXT_SYMBOL_PADDING
                );
            }
        }
        report.push(backtrace);
    }

    report.iter().fold(String::new(), |mut acc, b| {
        acc.push_str(b);
        acc.push('\n');
        acc
    })
}