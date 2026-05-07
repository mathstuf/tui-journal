use anyhow::{anyhow, bail};
use ansi_to_tui::IntoText;
use ratatui::text::Text;
use tokio::process::Command;

/// Renders journal content through an external tool, capturing ANSI output
/// and converting it to ratatui `Text` for display.
///
/// The `renderer_cmd` is a shell-style command string (e.g.
/// `"bat --language=md --paging=never --color=always"`).
/// The entry content is piped to stdin and the ANSI-colored stdout is captured.
pub async fn render_content(content: &str, renderer_cmd: &str) -> anyhow::Result<Text<'static>> {
    let mut parts = renderer_cmd.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow!("content_renderer command is empty"))?;
    let args: Vec<&str> = parts.collect();

    let mut child = Command::new(program)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| {
            anyhow!(
                "Failed to spawn content renderer '{}': {}",
                renderer_cmd,
                err
            )
        })?;

    // Write content to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(content.as_bytes()).await.map_err(|err| {
            anyhow!("Failed to write to content renderer stdin: {}", err)
        })?;
        // Drop stdin to signal EOF
    }

    let output = child.wait_with_output().await.map_err(|err| {
        anyhow!("Failed to wait for content renderer: {}", err)
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Content renderer exited with status {}:\n{}",
            output.status,
            stderr
        );
    }

    let text = output
        .stdout
        .into_text()
        .map_err(|err| anyhow!("Failed to convert ANSI output to styled text: {}", err))?;

    Ok(text)
}
