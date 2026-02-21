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
    widgets::{Block, Borders, Cell, Row, Table, TableState, Paragraph, Gauge},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};
use std::collections::HashMap;

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
    table_state: TableState,
    should_quit: bool,
    metrics_client: reqwest::blocking::Client,
}

impl DashboardApp {
    pub fn new() -> Self {
        let registry = Registry::new();
        let tunnels = registry.list_active();
        let mut table_state = TableState::default();
        if !tunnels.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            registry,
            tunnels,
            metrics: HashMap::new(),
            table_state,
            should_quit: false,
            metrics_client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(200))
                .build()
                .unwrap(),
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
                        KeyCode::Char('q') => self.should_quit = true,
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
        self.tunnels = self.registry.list_active();
        if self.tunnels.is_empty() {
            self.table_state.select(None);
        } else if self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }

        // Fetch metrics for all tunnels (or at least the visible/selected ones)
        for tunnel in &self.tunnels {
            let url = format!("http://localhost:{}/metrics", tunnel.metrics_port);
            if let Ok(res) = self.metrics_client.get(&url).send() {
                if let Ok(text) = res.text() {
                    let mut rx: u64 = 0;
                    let mut tx: u64 = 0;
                    for line in text.lines() {
                        if line.starts_with("cloudflared_tunnel_rx_bytes") {
                            rx = line.split_whitespace().last().and_then(|v| v.parse().ok()).unwrap_or(0);
                        } else if line.starts_with("cloudflared_tunnel_tx_bytes") {
                            tx = line.split_whitespace().last().and_then(|v| v.parse().ok()).unwrap_or(0);
                        }
                    }

                    let entry = self.metrics.entry(tunnel.pid).or_default();
                    if let Some(last) = entry.last_update {
                        let elapsed = last.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            entry.rx_speed = ((rx.saturating_sub(entry.rx_bytes)) as f64 / elapsed) as u64;
                            entry.tx_speed = ((tx.saturating_sub(entry.tx_bytes)) as f64 / elapsed) as u64;
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
        if self.tunnels.is_empty() { return; }
        let i = match self.table_state.selected() {
            Some(i) => if i >= self.tunnels.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.tunnels.is_empty() { return; }
        let i = match self.table_state.selected() {
            Some(i) => if i == 0 { self.tunnels.len() - 1 } else { i - 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn ui(&mut self, f: &mut ratatui::Frame) {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(10),
                Constraint::Length(3)
            ])
            .split(f.size());

        // Header
        let header = Paragraph::new(" xpose dashboard - Monitoring Hub")
            .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan)));
        f.render_widget(header, rects[0]);

        // Tunnels Table
        let selected_style = Style::default().add_modifier(Modifier::REVERSED).fg(Color::Yellow);
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
            let rx = m.map(|m| crate::ui::Ui::format_size(m.rx_speed) + "/s").unwrap_or_else(|| "0 B/s".to_string());
            let tx = m.map(|m| crate::ui::Ui::format_size(m.tx_speed) + "/s").unwrap_or_else(|| "0 B/s".to_string());
            let cells = vec![
                Cell::from(item.pid.to_string()),
                Cell::from(item.port.to_string()),
                Cell::from(rx),
                Cell::from(tx),
                Cell::from(item.url.clone()),
            ];
            Row::new(cells).height(1)
        });

        let t = Table::new(rows, [
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Min(30),
        ])
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Active Tunnels "))
        .highlight_style(selected_style)
        .highlight_symbol(">> ");

        f.render_stateful_widget(t, rects[1], &mut self.table_state);

        // Details / Metrics Panel
        if let Some(i) = self.table_state.selected() {
            if let Some(tunnel) = self.tunnels.get(i) {
                let m = self.metrics.get(&tunnel.pid);
                let rx_total = m.map(|m| crate::ui::Ui::format_size(m.rx_bytes)).unwrap_or_else(|| "0 B".to_string());
                let tx_total = m.map(|m| crate::ui::Ui::format_size(m.tx_bytes)).unwrap_or_else(|| "0 B".to_string());
                
                let detail_area = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rects[2]);

                let info = format!(
                    " Tunnel: {}\n Port: {}\n Protocol: {}\n Total Rx: {}\n Total Tx: {}",
                    tunnel.url, tunnel.port, tunnel.protocol, rx_total, tx_total
                );
                let details = Paragraph::new(info)
                    .block(Block::default().borders(Borders::ALL).title(" Tunnel Details "));
                f.render_widget(details, detail_area[0]);

                // Simple "Load" Gauge (example: proximity to a hypothetical 1GB limit)
                let total_bytes = m.map(|m| m.rx_bytes + m.tx_bytes).unwrap_or(0);
                let percentage = (total_bytes as f64 / 1_073_741_824.0 * 100.0).min(100.0) as u16;
                let gauge = Gauge::default()
                    .block(Block::default().borders(Borders::ALL).title(" Usage (vs 1GB Limit) "))
                    .gauge_style(Style::default().fg(if percentage > 80 { Color::Red } else { Color::Green }))
                    .percent(percentage);
                f.render_widget(gauge, detail_area[1]);
            }
        }

        // Footer
        let footer = Paragraph::new(" [Q] Quit  [↑/↓] Navigate")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, rects[3]);
    }
}
