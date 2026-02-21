use arboard::Clipboard;
use console::{Term, style};
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

    pub fn draw_connected_panel(&self, local_port: u16, public_url: &str, protocol: &str) {
        let _ = self.term.clear_screen();

        // Try to copy to clipboard
        let mut clipboard_msg = "(Auto-copy failed)";
        if let Ok(mut clipboard) = Clipboard::new()
            && clipboard.set_text(public_url.to_string()).is_ok()
        {
            clipboard_msg = "(Copied to clipboard)";
        }

        let border = style("=====================================================").cyan();

        let _ = self.term.write_line(&format!("{}", border));
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Tunnel").bold(),
            style("ACTIVE").green().bold()
        ));
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Local").bold(),
            style(format!("localhost:{}", local_port)).yellow()
        ));
        let _ = self.term.write_line(&format!(
            "  {} : {} {}",
            style("Public URL").bold(),
            style(public_url).green(),
            style(clipboard_msg).bright().black()
        ));
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Protocol").bold(),
            protocol.to_uppercase()
        ));
        let _ = self.term.write_line(&format!("{}", border));

        // Generate and draw QR Code using simple string renderer
        if let Ok(code) = QrCode::new(public_url) {
            let string = code
                .render::<char>()
                .quiet_zone(false)
                .dark_color('█')
                .light_color(' ')
                .build();
            let _ = self.term.write_line("  QR Code for Public URL:");
            for line in string.lines() {
                let _ = self.term.write_line(&format!("  {}", line));
            }
        }

        let _ = self.term.write_line(&format!(
            "\n  {} Press Ctrl+C to disconnect\n",
            style("Hint:").bright().black()
        ));
    }

    pub fn draw_live_metrics(
        &mut self,
        rx_bytes: u64,
        tx_bytes: u64,
        rx_speed: u64,
        tx_speed: u64,
        ping_ms: u64,
    ) {
        let _ = self.term.clear_line();

        // Update history for sparkline
        self.metrics_history.push(rx_speed + tx_speed);
        if self.metrics_history.len() > 20 {
            self.metrics_history.remove(0);
        }

        let sparkline = self.generate_sparkline();

        let total_mb = (rx_bytes + tx_bytes) as f64 / 1_048_576.0;
        let rx_kbps = rx_speed as f64 / 1024.0;
        let tx_kbps = tx_speed as f64 / 1024.0;

        let live_line = format!(
            "{} [{}] | {} {} | {} {} | {} {:.2} MB | {} {}ms",
            style("Flow:").dim(),
            style(sparkline).cyan(),
            style("↓ Rx:").cyan(),
            style(format!("{:.1} KB/s", rx_kbps)).bold(),
            style("↑ Tx:").magenta(),
            style(format!("{:.1} KB/s", tx_kbps)).bold(),
            style("Total:").yellow(),
            total_mb,
            style("Ping:").dim(),
            style(ping_ms.to_string()).bold()
        );

        let _ = self.term.write_str(&format!(
            "\r  {} {}",
            style("● Live Traffic:").green(),
            live_line
        ));
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
