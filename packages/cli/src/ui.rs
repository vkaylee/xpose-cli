use crate::i18n::I18n;
use arboard::Clipboard;
use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};
use qrcode::QrCode;
use std::time::Duration;

pub struct Ui {
    term: Term,
    metrics_history: Vec<u64>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
            metrics_history: Vec::with_capacity(20),
        }
    }

    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(message.to_string());
        pb
    }

    pub fn success(&self, msg: &str) {
        let _ = self
            .term
            .write_line(&format!("{} {}", style("✓").green().bold(), msg));
    }

    pub fn error(&self, msg: &str) {
        let _ = self
            .term
            .write_line(&format!("{} {}", style("✗").red().bold(), msg));
    }

    pub fn info(&self, msg: &str) {
        let _ = self
            .term
            .write_line(&format!("{} {}", style("i").cyan().bold(), msg));
    }

    pub fn draw_connected_panel(&self, port: u16, public_url: &str, protocol: &str, i18n: &I18n) {
        println!();
        println!(
            "  {} {}",
            Emoji("🚀", ">>"),
            style(i18n.t("connected")).green().bold().underlined()
        );

        // Try to copy to clipboard
        let mut clipboard_msg = "(Auto-copy failed)";
        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(public_url.to_string()).is_ok() {
                clipboard_msg = if i18n.lang == crate::i18n::Language::Vi {
                    "(Đã sao chép)"
                } else {
                    "(Copied to clipboard)"
                };
            }
        }

        let border = style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").cyan();

        let _ = self.term.write_line(&format!("{border}"));
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Tunnel").bold().blue(),
            style("ACTIVE").green().bold()
        ));
        let _ = self.term.write_line(&format!(
            "  {}  : {}",
            style("Local").bold().blue(),
            style(format!("localhost:{port}")).yellow().bold()
        ));
        let _ = self.term.write_line(&format!(
            "  {} : {} {}",
            style("Public").bold().blue(),
            style(public_url).green().underlined(),
            style(clipboard_msg).bright().black().italic()
        ));
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Protocol").bold().blue(),
            style(protocol.to_uppercase()).magenta().bold()
        ));
        let _ = self.term.write_line(&format!("{border}"));

        // Generate and draw QR Code using simple string renderer
        if let Ok(code) = QrCode::new(public_url) {
            let string = code
                .render::<char>()
                .quiet_zone(false)
                .dark_color('█')
                .light_color(' ')
                .build();
            let _ = self
                .term
                .write_line(&format!("  {} QR Code:", style("▶").cyan()));
            for line in string.lines() {
                let _ = self.term.write_line(&format!("  {line}"));
            }
        }

        let _ = self.term.write_line(&format!(
            "\n  {} {}\n",
            style("💡 Hint:").yellow().italic(),
            style(i18n.t("cli_help")).bright().black()
        ));
    }

    pub fn draw_live_metrics(
        &mut self,
        rx_bytes: u64,
        tx_bytes: u64,
        rx_speed: u64,
        tx_speed: u64,
        ping_ms: u64,
        ram_bytes: u64,
    ) {
        let _ = self.term.clear_line();

        // Update history for sparkline
        self.metrics_history.push(rx_speed + tx_speed);
        if self.metrics_history.len() > 20 {
            self.metrics_history.remove(0);
        }

        let sparkline = self.generate_sparkline();

        let total_formatted = Self::format_size(rx_bytes + tx_bytes);
        let rx_formatted = Self::format_size(rx_speed);
        let tx_formatted = Self::format_size(tx_speed);
        let ram_formatted = Self::format_size(ram_bytes);

        let live_line = format!(
            "{} [{}] | {} {}/s | {} {}/s | {} {} | {} {}ms | {} {}",
            style("Flow:").dim(),
            style(sparkline).cyan(),
            style("↓ Rx:").cyan(),
            style(rx_formatted).bold(),
            style("↑ Tx:").magenta(),
            style(tx_formatted).bold(),
            style("Total:").yellow(),
            total_formatted,
            style("Ping:").dim(),
            style(ping_ms.to_string()).bold(),
            style("RAM:").dim(),
            style(ram_formatted).bold()
        );

        let _ = self.term.write_str(&format!(
            "\r  {} {}",
            style("● Live Traffic:").green(),
            live_line
        ));
    }

    pub fn format_size(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        let bytes_f = bytes as f64;

        if bytes_f >= GB {
            format!("{:.2} GB", bytes_f / GB)
        } else if bytes_f >= MB {
            format!("{:.2} MB", bytes_f / MB)
        } else if bytes_f >= KB {
            format!("{:.1} KB", bytes_f / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    fn generate_sparkline(&self) -> String {
        if self.metrics_history.is_empty() {
            return " ".repeat(20);
        }

        let max = *self.metrics_history.iter().max().unwrap_or(&1);
        let blocks = [" ", " ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

        let mut line = String::new();
        for &val in &self.metrics_history {
            let idx = (val * (blocks.len() - 1) as u64 / max.max(1)) as usize;
            line.push_str(blocks[idx]);
        }

        // Pad with spaces if less than 20
        let current_count = line.chars().count();
        if current_count < 20 {
            line.push_str(&" ".repeat(20 - current_count));
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(Ui::format_size(500), "500 B");
        assert_eq!(Ui::format_size(1024), "1.0 KB");
        assert_eq!(Ui::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(Ui::format_size(1024 * 1024 * 1024), "1.00 GB");
    }
}
