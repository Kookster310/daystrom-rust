use crate::app::App;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Table, Wrap,
    },
    Frame, Terminal,
};
use std::io;
use tokio::time::{Duration, Instant};

pub async fn run_app(mut app: App) -> Result<()> {
    // Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app_internal(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app_internal<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Char('h') => {
                        app.toggle_help();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next_item();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.previous_item();
                    }
                    KeyCode::Char('r') => {
                        // Trigger manual refresh
                        app.update_statuses().await;
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update_statuses().await;
            last_tick = Instant::now();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(4),  // Title (increased for clock)
                Constraint::Length(3),  // Stats
                Constraint::Min(0),     // Main content
                Constraint::Length(3),  // Help/Status
            ]
            .as_ref(),
        )
        .split(f.size());

    render_title(f, app, chunks[0]);
    render_stats(f, app, chunks[1]);
    
    if app.show_help {
        render_help(f, chunks[2]);
    } else {
        render_services_table(f, app, chunks[2]);
    }
    
    render_status_bar(f, app, chunks[3]);
}

fn render_title(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let now = chrono::Utc::now();
    let timezone = &app.config.settings.timezone;
    
    // Try to parse the timezone, fallback to UTC if invalid
    let formatted_time = match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => now.with_timezone(&tz).format("%H:%M:%S %Z"),
        Err(_) => now.format("%H:%M:%S UTC"),
    };
    
    let last_update_formatted = match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => app.last_update.with_timezone(&tz).format("%H:%M:%S"),
        Err(_) => app.last_update.format("%H:%M:%S"),
    };
    
    let clock_text = format!("🕐 {} | Last Update: {}", 
        formatted_time,
        last_update_formatted);
    
    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "DAYSTROM TUI MONITORING DASHBOARD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                clock_text,
                Style::default().fg(Color::Gray),
            ),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).style(Style::default()))
    .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(title, area);
}

fn render_stats(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (up, down, unknown) = app.get_summary_stats();
    let total = app.get_total_services();
    let hosts = app.get_host_count();

    let stats_text = vec![
        Line::from(vec![
            Span::styled("🟢 UP: ", Style::default().fg(Color::Green)),
            Span::styled(format!("{}", up), Style::default().fg(Color::Green)),
            Span::styled("  ", Style::default()),
            Span::styled("🔴 DOWN: ", Style::default().fg(Color::Red)),
            Span::styled(format!("{}", down), Style::default().fg(Color::Red)),
            Span::styled("  ", Style::default()),
            Span::styled("🟡 UNKNOWN: ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("{}", unknown), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Total Services: ", Style::default().fg(Color::Blue)),
            Span::styled(format!("{}", total), Style::default().fg(Color::Blue)),
            Span::styled("  ", Style::default()),
            Span::styled("Hosts: ", Style::default().fg(Color::Blue)),
            Span::styled(format!("{}", hosts), Style::default().fg(Color::Blue)),
            Span::styled("  ", Style::default()),
            Span::styled("Refresh: ", Style::default().fg(Color::Blue)),
            Span::styled(
                format!("{}s", app.get_refresh_interval().as_secs()),
                Style::default().fg(Color::Blue),
            ),
        ]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Statistics"))
        .wrap(Wrap { trim: true });

    f.render_widget(stats, area);
}

fn render_services_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let statuses = app.get_status_list();
    
    if statuses.is_empty() {
        let no_data = Paragraph::new("No services configured or no data available yet...")
            .block(Block::default().borders(Borders::ALL).title("Services"))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(no_data, area);
        return;
    }

    let rows: Vec<Row> = statuses
        .iter()
        .enumerate()
        .map(|(index, status)| {
            let is_selected = index == app.selected_index;
            
            let _status_color = match status.status {
                crate::monitor::ServiceStatus::Up => Color::Green,
                crate::monitor::ServiceStatus::Down => Color::Red,
                crate::monitor::ServiceStatus::Unknown => Color::Yellow,
            };

            let response_time = if status.response_time.as_millis() > 0 {
                format!("{}ms", status.response_time.as_millis())
            } else {
                "N/A".to_string()
            };

            let error_msg = status.error_message.as_deref().unwrap_or("");

            Row::new(vec![
                Cell::from(format!("{}", status.host_name)),
                Cell::from(format!("{}:{}", status.service_name, status.port)),
                Cell::from(format!("{}", status.protocol)),
                Cell::from(format!("{}", status.status)),
                Cell::from(response_time),
                Cell::from(error_msg),
            ])
            .style(if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                Style::default()
            })
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            Constraint::Length(20),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(15),
            Constraint::Min(20),
        ]
    )
    .header(
        Row::new(vec![
            "Host",
            "Service",
            "Protocol",
            "Status",
            "Response Time",
            "Error",
        ])
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title("Services"))
    .column_spacing(1);

    f.render_widget(table, area);
}

fn render_help(f: &mut Frame, area: ratatui::layout::Rect) {
    let help_text = vec![
        Line::from(vec![
            Span::styled("Navigation: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("↑/k ", Style::default().fg(Color::Yellow)),
            Span::styled("- Previous item", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("↓/j ", Style::default().fg(Color::Yellow)),
            Span::styled("- Next item", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("h ", Style::default().fg(Color::Yellow)),
            Span::styled("- Toggle help", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("r ", Style::default().fg(Color::Yellow)),
            Span::styled("- Manual refresh", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("q/ESC ", Style::default().fg(Color::Yellow)),
            Span::styled("- Quit", Style::default()),
        ]),
    ];

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}

fn render_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let now = chrono::Utc::now();
    let timezone = &app.config.settings.timezone;
    
    // Try to parse the timezone, fallback to UTC if invalid
    let formatted_time = match timezone.parse::<chrono_tz::Tz>() {
        Ok(tz) => now.with_timezone(&tz).format("%H:%M:%S %Z"),
        Err(_) => now.format("%H:%M:%S UTC"),
    };
    
    let status_text = if app.show_help {
        format!("🕐 {} | Press 'h' to hide help | Press 'q' to quit", formatted_time)
    } else {
        format!("🕐 {} | Press 'h' for help | Press 'q' to quit | Press 'r' to refresh", formatted_time)
    };

    let status = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(status, area);
} 