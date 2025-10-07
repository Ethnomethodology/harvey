use tauri::Runtime;
use tauri_plugin_shell::Shell;
use crate::welcome::config::CommandError;

async fn run_check<R: Runtime>(shell: &Shell<R>, command: &str, args: &[&str]) -> Result<bool, CommandError> {
    log::info!("Running check: {} {}", command, args.join(" "));
    match shell.command(command).args(args).output().await {
        Ok(output) => {
            if output.status.success() {
                log::info!("'{}' found.", command);
                Ok(true)
            } else {
                log::warn!(
                    "'{}' command failed with status: {:?}. Stderr: {}",
                    command,
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(false)
            }
        }
        Err(e) => {
            log::warn!("Failed to execute '{}' command: {}. It might not be installed or not in the PATH.", command, e);
            Ok(false)
        }
    }
}

pub async fn check_pandoc_installed<R: Runtime>(shell: &Shell<R>) -> Result<bool, CommandError> {
    log::info!("Checking for Pandoc installation...");
    run_check(shell, "pandoc", &["--version"]).await
}

pub async fn check_ffmpeg_installed<R: Runtime>(shell: &Shell<R>) -> Result<bool, CommandError> {
    log::info!("Checking for FFmpeg installation...");
    run_check(shell, "ffmpeg", &["-version"]).await
}

pub async fn check_python_installed<R: Runtime>(shell: &Shell<R>) -> Result<bool, CommandError> {
    log::info!("Checking for Python 3 installation...");
    if run_check(shell, "python3", &["--version"]).await? {
        Ok(true)
    } else {
        log::info!("'python3' not found, trying 'python'...");
        run_check(shell, "python", &["--version"]).await
    }
}
