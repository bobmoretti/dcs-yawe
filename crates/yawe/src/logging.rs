use fern::colors::{Color, ColoredLevelConfig};
use std::fs::File;
use std::io::Write;
use std::os::windows::io::FromRawHandle;
use std::path::Path;
use windows::Win32::System::Console;

pub fn init(config: &config::Config) {
    let mut console_out = create_console().expect("Created console");
    writeln!(
        console_out,
        "Console creation complete, setting up logging."
    )
    .unwrap();

    setup_logging(&config, console_out).expect("Couldn't set up logging, very sad.");
}

fn create_console() -> windows::core::Result<File> {
    unsafe {
        Console::AllocConsole();
        let h_stdout = Console::GetStdHandle(Console::STD_OUTPUT_HANDLE)?;
        Ok(File::from_raw_handle(h_stdout.0 as *mut libc::c_void))
    }
}

fn setup_logging(config: &config::Config, mut console: File) -> Result<(), fern::InitError> {
    writeln!(console, "Setting up logging").unwrap();
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        // we actually don't need to specify the color for debug and info, they are white by default
        .info(Color::White)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    let colors_level = colors_line.clone().info(Color::Green);

    use log::LevelFilter;
    let level = if config.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let logdir = Path::new(config.write_dir.as_str())
        .join("Logs")
        .join("Yawe");

    std::fs::create_dir_all(&logdir).unwrap();
    let p = logdir.join("dcs_yawe.log");

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{thread_id}][{target}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                thread_id = thread_id::get(),
                target = record.target(),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        .level(level)
        .level_for("wgpu_core", LevelFilter::Warn)
        .level_for("naga", LevelFilter::Info)
        .chain(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(p)?,
        )
        .chain(console)
        .apply()?;

    log_panics::init();
    log::info!("Initialization of logging complete!");

    Ok(())
}
