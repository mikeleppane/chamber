use chamber_backup::BackgroundService;
use chamber_secret_manager_cli::Cli;
use chamber_secret_manager_cli::handle_command;
use chamber_ui::{App, run_app};
use chamber_vault::Vault;
use clap::Parser;
use color_eyre::Result;
#[cfg(not(windows))]
use jemallocator::Jemalloc;
#[cfg(windows)]
use mimalloc::MiMalloc;

#[cfg(windows)]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[cfg(not(windows))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> Result<()> {
    // If CLI arguments are provided, run CLI mode; otherwise, launch TUI.
    color_eyre::install()?;
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        match handle_command(cmd).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!("❌ Error: invalid command! Please check your input.\n{err}");
                std::process::exit(1);
            }
        }
    }

    let vault = Vault::open_default()?;
    let backup_config = vault.get_backup_config().unwrap_or_default();

    if backup_config.enabled {
        let background_service = BackgroundService::new(vault, backup_config);
        background_service.start();
    }

    // TUI mode
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let res = {
        let mut app = App::new()?;
        run_app(&mut app).await
    };

    crossterm::execute!(
        stdout,
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;

    res
}
