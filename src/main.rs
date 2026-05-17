mod gui;

use anyhow::{Context, Result};
use clap::Parser;
use cpp_rust_transpiler::{transpile, Direction};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "transpiler")]
#[command(about = "Bidirectional C++ ↔ Rust source-to-source transpiler")]
struct Cli {
    /// Source language
    #[arg(long = "from", value_name = "LANG")]
    from: String,

    /// Target language
    #[arg(long = "to", value_name = "LANG")]
    to: String,

    /// Input file
    input: PathBuf,

    /// Output file (defaults to stdout)
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<PathBuf>,
}

#[cfg(windows)]
fn hide_console_window() {
    use windows_sys::Win32::System::Console::FreeConsole;
    unsafe {
        FreeConsole();
    }
}

#[cfg(not(windows))]
fn hide_console_window() {}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // If no CLI arguments provided (just the exe name), launch GUI mode
    if args.len() <= 1 {
        hide_console_window();
        gui::run_gui().map_err(|e| anyhow::anyhow!(e.to_string()))?;
        return Ok(());
    }

    let cli = Cli::parse();

    let direction = match (cli.from.as_str(), cli.to.as_str()) {
        ("cpp", "rust") => Direction::CppToRust,
        ("rust", "cpp") => Direction::RustToCpp,
        _ => anyhow::bail!(
            "Unsupported transpilation direction: {} -> {}. Supported: cpp -> rust, rust -> cpp",
            cli.from,
            cli.to
        ),
    };

    let source = fs::read_to_string(&cli.input)
        .with_context(|| format!("Failed to read input file: {:?}", cli.input))?;

    let result = transpile(&source, direction)
        .with_context(|| "Transpilation failed")?;

    if let Some(out_path) = cli.output {
        fs::write(&out_path, result)
            .with_context(|| format!("Failed to write output file: {:?}", out_path))?;
        eprintln!("Successfully wrote output to {:?}", out_path);
    } else {
        println!("{}", result);
    }

    Ok(())
}
