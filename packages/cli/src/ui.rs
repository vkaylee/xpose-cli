use crate::i18n::I18n;
use arboard::Clipboard;
use console::{style, Emoji, Term};
use indicatif::{ProgressBar, ProgressStyle};
use qrcode::QrCode;
use std::time::Duration;

/// Extract the port from a URL string. Returns the default port for the scheme
/// (443 for HTTPS, 80 for HTTP) if no explicit port is present.
fn extract_port_from_url(url: &str) -> u16 {
    // Strip scheme to find host:port
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("tcp://")
        .trim_start_matches("udp://");

    // host:port or host:port/path
    if let Some(colon_pos) = without_scheme.rfind(':') {
        let port_str = &without_scheme[colon_pos + 1..];
        // Take only digits (stop at '/' or any non-digit)
        let port_digits: String = port_str
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(p) = port_digits.parse::<u16>() {
            return p;
        }
    }

    // Default ports based on scheme
    if url.starts_with("https://") {
        443
    } else if url.starts_with("http://") {
        80
    } else {
        0
    }
}

pub struct Ui {
    term: Term,
    metrics_history: Vec<u64>,
    pub i18n: I18n,
    pub silent: bool,
}

impl Ui {
    pub fn new(i18n: I18n) -> Self {
        Self {
            term: Term::stdout(),
            metrics_history: Vec::with_capacity(20),
            i18n,
            silent: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_silent(i18n: I18n) -> Self {
        Self {
            term: Term::stdout(),
            metrics_history: Vec::with_capacity(20),
            i18n,
            silent: true,
        }
    }

    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        if self.silent {
            return ProgressBar::hidden();
        }
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
        if self.silent {
            return;
        }
        let _ = self
            .term
            .write_line(&format!("{} {}", style("✓").green().bold(), msg));
    }

    pub fn error(&self, msg: &str) {
        if self.silent {
            return;
        }
        let _ = self
            .term
            .write_line(&format!("{} {}", style("✗").red().bold(), msg));
    }

    pub fn info(&self, msg: &str) {
        if self.silent {
            return;
        }
        let _ = self
            .term
            .write_line(&format!("{} {}", style("i").cyan().bold(), msg));
    }

    pub fn draw_connected_panel(&self, port: u16, public_url: &str, protocol: &str) {
        if self.silent {
            return;
        }
        let i18n = &self.i18n;
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

        // Extract and display the remote port from the public URL
        let remote_port = extract_port_from_url(public_url);
        let _ = self.term.write_line(&format!(
            "  {} : {}",
            style("Remote").bold().blue(),
            style(format!("port {remote_port}")).yellow().bold()
        ));

        let _ = self.term.write_line(&format!("{border}"));

        // Generate and draw QR Code using simple string renderer
        self.draw_qr(public_url, "QR Code:");

        let _ = self.term.write_line(&format!(
            "\n  {} {}\n",
            style("💡 Hint:").yellow().italic(),
            style(i18n.t("cli_help")).bright().black()
        ));
    }

    pub fn draw_auth_panel(&self) {
        if self.silent {
            return;
        }
        println!();
        println!(
            "  {} {}",
            Emoji("🔒", "!!"),
            style("SECURITY HANDSHAKE").yellow().bold()
        );
        println!(
            "  {}",
            style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim()
        );
    }

    pub fn draw_qr_auth(&self, url: &str) {
        self.draw_qr(url, "Scan to Authorize:");
    }

    fn draw_qr(&self, data: &str, label: &str) {
        if self.silent {
            return;
        }
        let _ = self
            .term
            .write_line(&format!("  {} {}", style("▶").cyan(), label));
        for line in Self::render_qr_lines(data) {
            let _ = self.term.write_line(&line);
        }
    }

    /// Renders a QR code as half-size terminal lines (2x2 modules per character).
    /// Returns empty vec if QR generation fails.
    fn render_qr_lines(data: &str) -> Vec<String> {
        let code = match QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::L) {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let width = code.width();
        let mut lines = Vec::new();

        // Each character represents 2x2 modules (half-size rendering)
        for y in (0..width).step_by(2) {
            let mut line = String::from("  ");
            for x in (0..width).step_by(2) {
                let top = code[(x, y)] == qrcode::Color::Dark;
                let bottom = if y + 1 < width {
                    code[(x, y + 1)] == qrcode::Color::Dark
                } else {
                    false
                };

                match (top, bottom) {
                    (true, true) => line.push('█'),
                    (true, false) => line.push('▀'),
                    (false, true) => line.push('▄'),
                    (false, false) => line.push(' '),
                }
            }
            lines.push(line);
        }
        lines
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
        if self.silent {
            return;
        }
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
        assert_eq!(Ui::format_size(2048), "2.0 KB");
    }

    #[test]
    fn test_generate_sparkline() {
        let i18n = crate::i18n::I18n::new(None);
        let mut ui = Ui::new(i18n);

        // Empty history
        assert_eq!(ui.generate_sparkline(), " ".repeat(20));

        // Mixed history
        ui.metrics_history = vec![0, 10, 20, 30, 40, 50, 60, 70, 80];
        let spark = ui.generate_sparkline();
        assert_eq!(spark.chars().count(), 20);
        assert!(spark.contains('█')); // Max value should be full block
        assert!(spark.contains(' ')); // Min value should be empty/low

        // All same values
        ui.metrics_history = vec![100; 20];
        let spark_constant = ui.generate_sparkline();
        assert_eq!(spark_constant, "████████████████████");
    }

    #[test]
    fn test_ui_drawing_methods_smoke() {
        let i18n = crate::i18n::I18n::new(None);
        let mut ui = Ui::new_silent(i18n);

        // Smoke tests for drawing methods (ensure no panic)
        ui.success("testing success");
        ui.error("testing error");
        ui.info("testing info");
        ui.draw_auth_panel();
        ui.draw_qr_auth("http://example.com");
        ui.draw_connected_panel(3000, "https://public.url", "tcp");
        ui.draw_live_metrics(100, 200, 10, 20, 15, 50 * 1024 * 1024);
    }

    #[test]
    fn test_ui_draw_qr_smoke() {
        let i18n = crate::i18n::I18n::new(None);
        let ui = Ui::new_silent(i18n.clone());
        // This should not panic
        ui.draw_qr_auth("https://xpose.dev/auth/v1/test-session-id");
    }

    #[test]
    fn test_ui_draw_panels_smoke() {
        let i18n = crate::i18n::I18n::new(None);
        let ui = Ui::new_silent(i18n.clone());

        ui.draw_auth_panel();
        ui.draw_connected_panel(8080, "https://test.xpose.dev", "tcp");

        let mut ui_mut = Ui::new_silent(i18n.clone());
        ui_mut.draw_live_metrics(1000, 2000, 100, 200, 10, 1024 * 1024);
    }

    #[test]
    fn test_ui_silent_mode_logic() {
        let i18n = crate::i18n::I18n::new(None);
        let ui_silent = Ui::new_silent(i18n.clone());
        assert!(ui_silent.silent);

        let ui_normal = Ui::new(i18n);
        assert!(!ui_normal.silent);
    }
    #[test]
    fn test_ui_silent_mode() {
        let i18n = I18n::new(None);
        let ui = Ui::new(i18n);
        assert!(!ui.silent);

        // These should not panic
        ui.success("test");
        ui.error("test");
        ui.info("test");
        ui.draw_connected_panel(80, "http://test", "tcp");
        ui.draw_auth_panel();
    }

    #[test]
    fn test_ui_draw_live_metrics() {
        let i18n = I18n::new(None);
        let mut ui = Ui::new(i18n);
        // Smoke test to ensure it doesn't panic with various values
        ui.draw_live_metrics(1024, 2048, 100, 200, 50, 1024 * 1024);
        ui.draw_live_metrics(0, 0, 0, 0, 0, 0);
        ui.draw_live_metrics(1_000_000, 2_000_000, 500_000, 500_000, 10, 100_000_000);
    }

    #[test]
    fn test_ui_draw_qr() {
        let i18n = I18n::new(None);
        let ui = Ui::new(i18n);
        // Smoke test for QR drawing
        ui.draw_qr("https://xpose.cloud/verify/123", "Scan to Verify");
    }

    #[test]
    fn test_render_qr_lines_half_size() {
        use qrcode::QrCode;

        let data = "https://xpose.cloud/verify/123";

        // Measure raw QR module dimensions
        let code =
            QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::L).unwrap();
        let module_width = code.width();

        // Rendered output
        let lines = Ui::render_qr_lines(data);

        // Row count: ceil(module_width / 2)
        let expected_rows = module_width.div_ceil(2);
        assert_eq!(
            lines.len(),
            expected_rows,
            "row count should be ceil(module_width / 2)"
        );

        // Column count per line: 2 (indent) + ceil(module_width / 2) chars
        let expected_cols = 2 + module_width.div_ceil(2);
        for line in &lines {
            assert_eq!(
                line.chars().count(),
                expected_cols,
                "each line width should be 2-indent + ceil(module_width / 2)"
            );
        }
    }

    #[test]
    fn test_render_qr_lines_invalid_data() {
        // Empty input can still produce a QR code; truly invalid data is hard to trigger,
        // so verify it doesn't panic and returns a non-empty result for valid input.
        let lines = Ui::render_qr_lines("https://xpose.cloud");
        assert!(!lines.is_empty(), "valid data should produce QR lines");
    }

    // ── extract_port_from_url ────────────────────────────────────────────

    #[test]
    fn test_extract_port_https_default() {
        assert_eq!(extract_port_from_url("https://abc.trycloudflare.com"), 443);
    }

    #[test]
    fn test_extract_port_http_default() {
        assert_eq!(extract_port_from_url("http://example.com"), 80);
    }

    #[test]
    fn test_extract_port_explicit() {
        assert_eq!(extract_port_from_url("https://example.com:8443"), 8443);
    }

    #[test]
    fn test_extract_port_tcp_explicit() {
        assert_eq!(extract_port_from_url("tcp://ssh.example.com:22"), 22);
    }

    #[test]
    fn test_extract_port_udp_explicit() {
        assert_eq!(extract_port_from_url("udp://game.example.com:27015"), 27015);
    }

    #[test]
    fn test_extract_port_no_scheme() {
        assert_eq!(extract_port_from_url("example.com:3000"), 3000);
    }

    #[test]
    fn test_extract_port_unknown_scheme_no_port() {
        assert_eq!(extract_port_from_url("tcp://example.com"), 0);
    }

    #[test]
    fn test_extract_port_with_path() {
        assert_eq!(extract_port_from_url("https://example.com:9090/path"), 9090);
    }
}
