use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use std::{io, sync::mpsc, thread, time::Duration};

#[derive(Clone, Debug, Default)]
pub struct ServerMetrics {
    pub current_tick: u64,
    pub tick_duration: Duration,
    pub connected_clients: usize,
    pub pending_chunks: usize,
    pub queue_size: usize,
}

pub fn spawn_tui_thread(rx: mpsc::Receiver<ServerMetrics>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut latest_metrics = ServerMetrics::default();

        loop {
            // 1. Drain channel to get the most recent snapshot
            while let Ok(metrics) = rx.try_recv() {
                latest_metrics = metrics;
            }

            // 2. Poll keyboard events (exit on 'q')
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }

            // 3. Render Dashboard
            let _ = terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Min(5),
                    ])
                    .split(f.size());

                let target_tick_ms = 50.0; // 20 TPS target (50ms)
                let actual_ms = latest_metrics.tick_duration.as_secs_f64() * 1000.0;
                let usage_ratio = (actual_ms / target_tick_ms).clamp(0.0, 1.0);
                let is_lagging = actual_ms > target_tick_ms;

                let status_title = if is_lagging {
                    format!(
                        "Server Metrics [TICK {} - LAGGING]",
                        latest_metrics.current_tick
                    )
                } else {
                    format!("Server Metrics [TICK {} - OK]", latest_metrics.current_tick)
                };

                let info_text = Paragraph::new(format!(
                    "Clients: {} | Pending Chunks: {} | Queued Chunks: {}",
                    latest_metrics.connected_clients,
                    latest_metrics.pending_chunks,
                    latest_metrics.queue_size
                ))
                .block(Block::default().borders(Borders::ALL).title(status_title));

                let tick_gauge = Gauge::default()
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Tick Duration (Target: 50.00ms)"),
                    )
                    .gauge_style(if is_lagging {
                        ratatui::style::Style::default().fg(ratatui::style::Color::Red)
                    } else {
                        ratatui::style::Style::default().fg(ratatui::style::Color::Green)
                    })
                    .percent((usage_ratio * 100.0) as u16)
                    .label(format!("{:.2}ms / 50.00ms", actual_ms));

                f.render_widget(info_text, chunks[0]);
                f.render_widget(tick_gauge, chunks[1]);
            });
        }

        // Clean up terminal state before exiting thread
        let _ = disable_raw_mode();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    })
}
