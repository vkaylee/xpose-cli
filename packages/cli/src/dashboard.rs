use crate::api::{ApiClient, GlobalStats};
use crate::i18n::I18n;
use crate::registry::{Registry, TunnelEntry};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, TableState},
    Terminal,
};
use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};

#[derive(Default, Clone)]
struct TunnelMetrics {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_speed: u64,
    tx_speed: u64,
    last_update: Option<Instant>,
}

pub struct DashboardApp {
    registry: Registry,
    tunnels: Vec<TunnelEntry>,
    metrics: HashMap<u32, TunnelMetrics>,
    global_stats: GlobalStats,
    table_state: TableState,
    should_quit: bool,
    metrics_client: reqwest::blocking::Client,
    api_client: ApiClient,
    tick_count: u64,
    i18n: I18n,
    sys: sysinfo::System,
    pid: sysinfo::Pid,
}

impl DashboardApp {
    pub fn new(api_url: String, i18n: I18n) -> Self {
        let registry = Registry::new();
        let tunnels = registry.list_active();
        let mut table_state = TableState::default();
        if !tunnels.is_empty() {
            table_state.select(Some(0));
        }
        let sys = sysinfo::System::new();
        let pid = sysinfo::Pid::from(std::process::id() as usize);

        Self {
            registry,
            tunnels,
            metrics: HashMap::new(),
            global_stats: GlobalStats::default(),
            table_state,
            should_quit: false,
            metrics_client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(200))
                .build()
                .unwrap(),
            api_client: ApiClient::new(api_url),
            tick_count: 0,
            i18n,
            sys,
            pid,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let tick_rate = Duration::from_millis(500);
        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| self.ui(f))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('x') => self.stop_selected_session(),
                        KeyCode::Char('r') => self.restart_selected_session(),
                        KeyCode::Down => self.next(),
                        KeyCode::Up => self.previous(),
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }

            if self.should_quit {
                break;
            }
        }

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }

    fn on_tick(&mut self) {
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::Some(&[self.pid]),
            false,
            sysinfo::ProcessRefreshKind::nothing().with_memory(),
        );
        self.tunnels = self.registry.list_active();
        if self.tunnels.is_empty() {
            self.table_state.select(None);
        } else if self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }

        // Fetch global stats every 10 ticks (approx every 5s if tick_rate is 500ms)
        if self.tick_count.is_multiple_of(10) {
            if let Ok(stats) =
                tokio::runtime::Handle::current().block_on(self.api_client.get_global_stats())
            {
                self.global_stats = stats;
            }
        }
        self.tick_count = self.tick_count.wrapping_add(1);

        // Fetch metrics for all tunnels
        for tunnel in &self.tunnels {
            let url = format!("http://localhost:{}/metrics", tunnel.metrics_port);
            if let Ok(res) = self.metrics_client.get(&url).send() {
                if let Ok(text) = res.text() {
                    let (rx, tx) = parse_metrics(&text);

                    let entry = self.metrics.entry(tunnel.pid).or_default();
                    if let Some(last) = entry.last_update {
                        let elapsed = last.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            entry.rx_speed =
                                ((rx.saturating_sub(entry.rx_bytes)) as f64 / elapsed) as u64;
                            entry.tx_speed =
                                ((tx.saturating_sub(entry.tx_bytes)) as f64 / elapsed) as u64;
                        }
                    }
                    entry.rx_bytes = rx;
                    entry.tx_bytes = tx;
                    entry.last_update = Some(Instant::now());
                }
            }
        }
    }

    fn next(&mut self) {
        if self.tunnels.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.tunnels.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.tunnels.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tunnels.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn stop_selected_session(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(tunnel) = self.tunnels.get(i) {
                let pid = tunnel.pid;
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGTERM);
                }
                let _ = self.registry.unregister(pid);
                self.on_tick();
            }
        }
    }

    fn restart_selected_session(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(tunnel) = self.tunnels.get(i) {
                let _pid = tunnel.pid;
                let _port = tunnel.port;
                let _protocol = tunnel.protocol.clone();

                // Stop current
                unsafe {
                    libc::kill(tunnel.pid as libc::pid_t, libc::SIGTERM);
                }
                let _ = self.registry.unregister(tunnel.pid);

                // Re-launch is handled by the user for now in this version,
                // but we trigger the registry update immediately.
                self.on_tick();
            }
        }
    }

    fn ui(&mut self, f: &mut ratatui::Frame) {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        // Header
        let ram_bytes = self.sys.process(self.pid).map(|p| p.memory()).unwrap_or(0);
        let ram_formatted = crate::ui::Ui::format_size(ram_bytes);

        let header_content = vec![
            ratatui::text::Span::styled(
                format!(" {} ", self.i18n.t("dashboard_title")),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::raw(" | "),
            ratatui::text::Span::styled(
                self.i18n
                    .t("global_stats")
                    .replace("{}", &self.global_stats.busy.to_string())
                    .replace("{}", &self.global_stats.available.to_string()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::raw(" | "),
            ratatui::text::Span::styled(
                format!("{}: {}", self.i18n.t("ram_usage"), ram_formatted),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        let header = Paragraph::new(ratatui::text::Line::from(header_content)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(header, rects[0]);

        // Tunnels Table
        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::Yellow);
        let normal_style = Style::default().fg(Color::White);
        let header_cells = ["PID", "Port", "Rx Speed", "Tx Speed", "Public URL"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Green)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);

        let rows = self.tunnels.iter().map(|item| {
            let m = self.metrics.get(&item.pid);
            let rx = m
                .map(|m| crate::ui::Ui::format_size(m.rx_speed) + "/s")
                .unwrap_or_else(|| "0 B/s".to_string());
            let tx = m
                .map(|m| crate::ui::Ui::format_size(m.tx_speed) + "/s")
                .unwrap_or_else(|| "0 B/s".to_string());

            let cells = vec![
                Cell::from(item.pid.to_string()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(item.port.to_string()).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(rx).style(Style::default().fg(Color::Cyan)),
                Cell::from(tx).style(Style::default().fg(Color::Magenta)),
                Cell::from(item.url.clone()).style(Style::default().fg(Color::Green)),
            ];
            Row::new(cells).height(1)
        });

        let t = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(15),
                Constraint::Length(15),
                Constraint::Min(30),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(self.i18n.t("active_tunnels"))
                .border_style(Style::default().fg(Color::Green)),
        )
        .highlight_style(selected_style)
        .highlight_symbol(" ❱ ");

        f.render_stateful_widget(t, rects[1], &mut self.table_state);

        // Details / Metrics Panel
        if let Some(i) = self.table_state.selected() {
            if let Some(tunnel) = self.tunnels.get(i) {
                let m = self.metrics.get(&tunnel.pid);
                let rx_total = m
                    .map(|m| crate::ui::Ui::format_size(m.rx_bytes))
                    .unwrap_or_else(|| "0 B".to_string());
                let tx_total = m
                    .map(|m| crate::ui::Ui::format_size(m.tx_bytes))
                    .unwrap_or_else(|| "0 B".to_string());

                let detail_area = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rects[2]);

                let info = vec![
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " URL: ",
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::styled(
                            &tunnel.url,
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " Port: ",
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::styled(
                            tunnel.port.to_string(),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " Protocol: ",
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::styled(
                            tunnel.protocol.to_uppercase(),
                            Style::default().fg(Color::Magenta),
                        ),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " Total Rx: ",
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::raw(rx_total),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            " Total Tx: ",
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        ),
                        ratatui::text::Span::raw(tx_total),
                    ]),
                ];

                let details = Paragraph::new(info).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(self.i18n.t("tunnel_details"))
                        .border_style(Style::default().fg(Color::Blue)),
                );
                f.render_widget(details, detail_area[0]);

                // Infrastructure usage gauge
                let global_tunnels = self.global_stats.total;
                let usage_percent = if global_tunnels > 0 {
                    (self.global_stats.busy as f64 / global_tunnels as f64 * 100.0) as u16
                } else {
                    0
                };

                let gauge = Gauge::default()
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(self.i18n.t("infra_usage"))
                            .border_style(Style::default().fg(if usage_percent > 80 {
                                Color::Red
                            } else {
                                Color::Green
                            })),
                    )
                    .gauge_style(
                        Style::default()
                            .fg(if usage_percent > 80 {
                                Color::Red
                            } else {
                                Color::Green
                            })
                            .add_modifier(Modifier::BOLD),
                    )
                    .percent(usage_percent);
                f.render_widget(gauge, detail_area[1]);
            }
        }

        // Footer
        let footer = Paragraph::new(self.i18n.t("footer_help"))
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(footer, rects[3]);
    }
}

pub fn parse_metrics(text: &str) -> (u64, u64) {
    let mut rx = 0;
    let mut tx = 0;
    for line in text.lines() {
        if line.starts_with("cloudflared_tunnel_rx_bytes") {
            rx = line
                .split_whitespace()
                .last()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("cloudflared_tunnel_tx_bytes") {
            tx = line
                .split_whitespace()
                .last()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }
    (rx, tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metrics() {
        let text = "cloudflared_tunnel_rx_bytes 1024\ncloudflared_tunnel_tx_bytes 2048";
        let (rx, tx) = parse_metrics(text);
        assert_eq!(rx, 1024);
        assert_eq!(tx, 2048);
    }

    #[test]
    fn test_dashboard_navigation() {
        let mut app = DashboardApp::new("http://localhost".to_string(), I18n::new(None));
        app.tunnels = vec![
            TunnelEntry {
                pid: 1,
                port: 3000,
                protocol: "tcp".to_string(),
                url: "u1".to_string(),
                start_time: 0,
                metrics_port: 0,
            },
            TunnelEntry {
                pid: 2,
                port: 8080,
                protocol: "tcp".to_string(),
                url: "u2".to_string(),
                start_time: 0,
                metrics_port: 0,
            },
        ];
        app.table_state.select(Some(0));

        app.next();
        assert_eq!(app.table_state.selected(), Some(1));

        app.next();
        assert_eq!(app.table_state.selected(), Some(0)); // Boundary wrap

        app.previous();
        assert_eq!(app.table_state.selected(), Some(1)); // Boundary wrap
    }
}
