use std::cmp::min;
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use app_services::{
    CaptureStatsInput, CaptureStatsService, ConversationsInput, ConversationsService,
    InspectCaptureInput, InspectCaptureService, InspectPacketInput, InspectPacketService,
    ListPacketsInput, ListPacketsService, LiveCaptureCoordinator, LiveCaptureSession,
    SaveCaptureInput, SaveCaptureService, StartLiveCaptureInput, StreamsInput, StreamsService,
    TransactionsInput, TransactionsService,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use output_formatters::{
    render_capture_stats_report, render_conversation_report, render_engine_info_report,
    render_packet_detail_report, render_stream_report, render_transaction_report,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use session_model::{
    CaptureReport, CaptureStatsReport, ConversationReport, PacketDetailReport, PacketListReport,
    StreamReport, TransactionReport,
};

use crate::{engine_info_report, filter_input::normalize_filter_expression, render_capture_error};

const REFRESH_TICK: Duration = Duration::from_millis(125);
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(900);

pub fn run_app(path: Option<PathBuf>) -> Result<(), String> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .map_err(|error| format!("failed to initialize terminal backend: {error}"))?;
    let _guard = TerminalGuard::enter(&mut terminal)?;
    let mut app = CliApp::new(path);

    app.refresh_interfaces();
    if let Some(path) = app.current_path.clone() {
        app.open_capture(path);
    } else {
        app.refresh_visible_section();
    }

    loop {
        terminal
            .draw(|frame| app.render(frame))
            .map_err(|error| format!("failed to draw terminal UI: {error}"))?;

        if event::poll(REFRESH_TICK).map_err(|error| format!("failed to poll input: {error}"))? {
            match event::read().map_err(|error| format!("failed to read input: {error}"))? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if !app.handle_key(key)? {
                        break;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        app.on_tick();
    }

    app.shutdown();
    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Self, String> {
        enable_raw_mode().map_err(|error| format!("failed to enable raw mode: {error}"))?;
        execute!(io::stdout(), EnterAlternateScreen)
            .map_err(|error| format!("failed to enter alternate screen: {error}"))?;
        terminal
            .hide_cursor()
            .map_err(|error| format!("failed to hide terminal cursor: {error}"))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Packets,
    Stats,
    Conversations,
    Streams,
    Transactions,
    Engine,
}

impl Section {
    const ALL: [Section; 6] = [
        Section::Packets,
        Section::Stats,
        Section::Conversations,
        Section::Streams,
        Section::Transactions,
        Section::Engine,
    ];

    fn title(self) -> &'static str {
        match self {
            Section::Packets => "Packets",
            Section::Stats => "Stats",
            Section::Conversations => "Conversations",
            Section::Streams => "Streams",
            Section::Transactions => "Transactions",
            Section::Engine => "Engine",
        }
    }

    fn accent(self) -> Color {
        match self {
            Section::Packets => Color::Cyan,
            Section::Stats => Color::LightBlue,
            Section::Conversations => Color::LightMagenta,
            Section::Streams => Color::LightGreen,
            Section::Transactions => Color::LightYellow,
            Section::Engine => Color::LightRed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    OpenCapture,
    Filter,
    SaveCapture,
}

impl PromptKind {
    fn title(self) -> &'static str {
        match self {
            PromptKind::OpenCapture => "Open Capture",
            PromptKind::Filter => "Packet Filter",
            PromptKind::SaveCapture => "Save Capture",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            PromptKind::OpenCapture => "Enter a capture file path and press Enter.",
            PromptKind::Filter => "Set a packet filter. Leave empty to clear it.",
            PromptKind::SaveCapture => "Enter an output path for the exported capture.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptState {
    kind: PromptKind,
    input: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusMessage {
    kind: MessageKind,
    text: String,
}

impl StatusMessage {
    fn info(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Info,
            text: text.into(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            kind: MessageKind::Error,
            text: text.into(),
        }
    }
}

struct CliApp {
    current_path: Option<PathBuf>,
    filter: String,
    section_index: usize,
    packet_selection: usize,
    packet_list_state: ListState,
    packet_detail_scroll: u16,
    stats_scroll: u16,
    conversations_scroll: u16,
    streams_scroll: u16,
    transactions_scroll: u16,
    engine_scroll: u16,
    capture_summary: Option<CaptureReport>,
    packet_list: Option<PacketListReport>,
    packet_detail: Option<PacketDetailReport>,
    stats_report: Option<CaptureStatsReport>,
    conversations_report: Option<ConversationReport>,
    streams_report: Option<StreamReport>,
    transactions_report: Option<TransactionReport>,
    available_interfaces: Vec<String>,
    selected_interface: usize,
    active_capture: Option<LiveCaptureSession>,
    capture_state_label: String,
    status: StatusMessage,
    prompt: Option<PromptState>,
    show_help: bool,
    last_live_refresh: Instant,
}

impl CliApp {
    fn new(path: Option<PathBuf>) -> Self {
        let mut packet_list_state = ListState::default();
        packet_list_state.select(Some(0));
        Self {
            current_path: path,
            filter: String::new(),
            section_index: 0,
            packet_selection: 0,
            packet_list_state,
            packet_detail_scroll: 0,
            stats_scroll: 0,
            conversations_scroll: 0,
            streams_scroll: 0,
            transactions_scroll: 0,
            engine_scroll: 0,
            capture_summary: None,
            packet_list: None,
            packet_detail: None,
            stats_report: None,
            conversations_report: None,
            streams_report: None,
            transactions_report: None,
            available_interfaces: Vec::new(),
            selected_interface: 0,
            active_capture: None,
            capture_state_label: "idle".to_string(),
            status: StatusMessage::info(
                "Ready. Press o to open a capture, c to start live capture, and ? for help.",
            ),
            prompt: None,
            show_help: false,
            last_live_refresh: Instant::now(),
        }
    }

    fn current_section(&self) -> Section {
        Section::ALL[self.section_index]
    }

    fn current_filter(&self) -> Option<String> {
        normalize_filter_expression(&self.filter)
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let outer = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(72, 88, 120)))
            .style(Style::default().bg(Color::Rgb(9, 12, 20)));
        frame.render_widget(outer, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(12),
                Constraint::Length(2),
            ])
            .margin(1)
            .split(area);

        self.render_header(frame, layout[0]);
        self.render_body(frame, layout[1]);
        self.render_footer(frame, layout[2]);

        if self.show_help {
            self.render_help(frame, area);
        }

        if self.prompt.is_some() {
            self.render_prompt(frame, area);
        }
    }

    fn render_header(&self, frame: &mut Frame<'_>, area: Rect) {
        let header = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(38)])
            .split(area);

        let title_lines = vec![
            Line::from(vec![
                Span::styled(
                    "ICE",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "SNIFF",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    "CLI APP",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("capture ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.current_capture_label(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("filter ", Style::default().fg(Color::DarkGray)),
                Span::styled(self.filter_label(), Style::default().fg(Color::LightCyan)),
            ]),
        ];
        let title = Paragraph::new(title_lines).block(
            Block::default()
                .title(" Workspace ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
        );
        frame.render_widget(title, header[0]);

        let runtime = LiveCaptureCoordinator::default().runtime_info();
        let meta_lines = vec![
            Line::from(vec![
                Span::styled("section ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.current_section().title(),
                    Style::default()
                        .fg(self.current_section().accent())
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("interface ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.current_interface_label(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("live ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &self.capture_state_label,
                    Style::default()
                        .fg(if self.active_capture.is_some() {
                            Color::LightGreen
                        } else {
                            Color::Gray
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("backend ", Style::default().fg(Color::DarkGray)),
                Span::styled(runtime.backend.as_str(), Style::default().fg(Color::Gray)),
            ]),
        ];
        let meta = Paragraph::new(meta_lines).block(
            Block::default()
                .title(" Runtime ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
        );
        frame.render_widget(meta, header[1]);
    }

    fn render_body(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(40)])
            .split(area);

        self.render_nav(frame, body[0]);
        self.render_section(frame, body[1]);
    }

    fn render_nav(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let items = Section::ALL
            .iter()
            .map(|section| {
                ListItem::new(vec![
                    Line::from(Span::styled(
                        format!("  {}", section.title()),
                        Style::default()
                            .fg(section.accent())
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        section_hint(*section),
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Sections ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(19, 33, 56))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶");

        let mut nav_state = ListState::default();
        nav_state.select(Some(self.section_index));
        frame.render_stateful_widget(list, area, &mut nav_state);
    }

    fn render_section(&mut self, frame: &mut Frame<'_>, area: Rect) {
        match self.current_section() {
            Section::Packets => self.render_packets_section(frame, area),
            Section::Stats => self.render_text_section(
                frame,
                area,
                "Capture Stats",
                self.stats_text(),
                self.stats_scroll,
            ),
            Section::Conversations => self.render_text_section(
                frame,
                area,
                "Conversations",
                self.conversations_text(),
                self.conversations_scroll,
            ),
            Section::Streams => self.render_text_section(
                frame,
                area,
                "Streams",
                self.streams_text(),
                self.streams_scroll,
            ),
            Section::Transactions => self.render_text_section(
                frame,
                area,
                "Transactions",
                self.transactions_text(),
                self.transactions_scroll,
            ),
            Section::Engine => self.render_text_section(
                frame,
                area,
                "Engine Capabilities",
                render_engine_info_report(&engine_info_report()),
                self.engine_scroll,
            ),
        }
    }

    fn render_packets_section(&mut self, frame: &mut Frame<'_>, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(10)])
            .split(area);

        let summary = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
            .split(layout[0]);

        let summary_text = if let Some(report) = &self.capture_summary {
            vec![
                Line::from(vec![
                    Span::styled("path ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        report.path.display().to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("size ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{} bytes", report.size_bytes),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::raw("  "),
                    Span::styled("packets ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        report
                            .packet_count_hint
                            .map(|count| count.to_string())
                            .unwrap_or_else(|| "n/a".to_string()),
                        Style::default().fg(Color::LightCyan),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("actions ", Style::default().fg(Color::DarkGray)),
                    Span::raw("o open  f filter  s save"),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "No capture open.",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "Press o to open a file or c to start live capture.",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let summary_widget = Paragraph::new(summary_text).block(
            Block::default()
                .title(" Session ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
        );
        frame.render_widget(summary_widget, summary[0]);

        let controls = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("interface ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.current_interface_label(),
                    Style::default().fg(Color::White),
                ),
                Span::raw("  "),
                Span::styled("cycle ", Style::default().fg(Color::DarkGray)),
                Span::raw("i / I"),
            ]),
            Line::from(vec![
                Span::styled("capture ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if self.active_capture.is_some() {
                        "c stop live capture"
                    } else {
                        "c start live capture"
                    },
                    Style::default()
                        .fg(if self.active_capture.is_some() {
                            Color::LightRed
                        } else {
                            Color::LightGreen
                        })
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("selection ", Style::default().fg(Color::DarkGray)),
                Span::raw("j / k move packets"),
                Span::raw("  "),
                Span::styled("refresh ", Style::default().fg(Color::DarkGray)),
                Span::raw("r"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Capture Controls ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
        );
        frame.render_widget(controls, summary[1]);

        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(47), Constraint::Percentage(53)])
            .split(layout[1]);

        let packet_items = self
            .packet_list
            .as_ref()
            .map(|report| {
                report
                    .packets
                    .iter()
                    .map(|packet| {
                        ListItem::new(vec![
                            Line::from(vec![
                                Span::styled(
                                    format!("#{:04}", packet.summary.index),
                                    Style::default()
                                        .fg(Color::LightCyan)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::raw("  "),
                                Span::styled(
                                    truncate_text(&packet.protocol, 12),
                                    Style::default().fg(Color::LightMagenta),
                                ),
                            ]),
                            Line::from(Span::styled(
                                format!(
                                    "{} -> {}",
                                    truncate_text(&packet.source, 28),
                                    truncate_text(&packet.destination, 28)
                                ),
                                Style::default().fg(Color::Gray),
                            )),
                            Line::from(Span::raw(truncate_text(&packet.info, 72))),
                        ])
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![ListItem::new(vec![
                    Line::from(Span::styled(
                        "No packet list available.",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "Open a capture or start a live session.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])]
            });

        let packet_title = match &self.packet_list {
            Some(report) => format!(
                " Packets ({}/{}) ",
                report.packets.len(),
                report.total_packets
            ),
            None => " Packets ".to_string(),
        };
        let packets = List::new(packet_items)
            .block(
                Block::default()
                    .title(packet_title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(20, 38, 68))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▍");
        frame.render_stateful_widget(packets, panes[0], &mut self.packet_list_state);

        let detail_title = self
            .packet_detail
            .as_ref()
            .map(|report| format!(" Packet Detail #{:04} ", report.packet.summary.index))
            .unwrap_or_else(|| " Packet Detail ".to_string());
        let detail_text = self.packet_detail_text();
        let detail = Paragraph::new(detail_text)
            .block(
                Block::default()
                    .title(detail_title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.packet_detail_scroll, 0));
        frame.render_widget(detail, panes[1]);
    }

    fn render_text_section(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        title: &str,
        text: String,
        scroll: u16,
    ) {
        let text = if self.current_path.is_none() && self.current_section() != Section::Engine {
            format!(
                "{title}\n\nNo capture open.\n\nPress o to open a capture file or c to start live capture."
            )
        } else {
            text
        };
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(48, 62, 91))),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame<'_>, area: Rect) {
        let style = match self.status.kind {
            MessageKind::Info => Style::default().fg(Color::Gray),
            MessageKind::Error => Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        };

        let footer = Paragraph::new(Line::from(vec![
            Span::styled(&self.status.text, style),
            Span::raw("  "),
            Span::styled(
                "keys: q quit  ←/→ section  j/k move  o open  f filter  s save  c capture  ? help",
                Style::default().fg(Color::DarkGray),
            ),
        ]))
        .alignment(Alignment::Left);
        frame.render_widget(footer, area);
    }

    fn render_help(&self, frame: &mut Frame<'_>, area: Rect) {
        let popup = centered_rect(58, 60, area);
        frame.render_widget(Clear, popup);
        let lines = vec![
            Line::from(Span::styled(
                "IceSniff CLI App",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Navigation"),
            Line::from("  left/right or tab: switch sections"),
            Line::from("  j / k: move packet selection or scroll section text"),
            Line::from("  g / G: jump to top or bottom"),
            Line::from(""),
            Line::from("Actions"),
            Line::from("  o: open capture file"),
            Line::from("  f: set or clear packet filter"),
            Line::from("  s: save current capture"),
            Line::from("  i / I: next or previous network interface"),
            Line::from("  c: start or stop live capture"),
            Line::from("  r: refresh current section"),
            Line::from(""),
            Line::from("General"),
            Line::from("  ?: toggle this help"),
            Line::from("  esc: close overlays"),
            Line::from("  q: quit"),
        ];
        let widget = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Help ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, popup);
    }

    fn render_prompt(&self, frame: &mut Frame<'_>, area: Rect) {
        let Some(prompt) = &self.prompt else {
            return;
        };
        let popup = centered_rect(62, 22, area);
        frame.render_widget(Clear, popup);
        let widget = Paragraph::new(vec![
            Line::from(Span::styled(
                prompt.kind.hint(),
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &prompt.input,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Enter to submit, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .title(format!(" {} ", prompt.kind.title()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::LightCyan)),
        )
        .wrap(Wrap { trim: false });
        frame.render_widget(widget, popup);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool, String> {
        if self.prompt.is_some() {
            return self.handle_prompt_key(key);
        }

        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => self.show_help = false,
                KeyCode::Char('q') => return Ok(false),
                _ => {}
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('q') => return Ok(false),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Left => self.previous_section(),
            KeyCode::Right | KeyCode::Tab => self.next_section(),
            KeyCode::BackTab => self.previous_section(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::PageUp => self.scroll_detail_up(),
            KeyCode::PageDown => self.scroll_detail_down(),
            KeyCode::Char('h') => self.previous_section(),
            KeyCode::Char('l') => self.next_section(),
            KeyCode::Char('j') => self.move_down(),
            KeyCode::Char('k') => self.move_up(),
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::SHIFT) => self.go_bottom(),
            KeyCode::Char('g') => self.go_top(),
            KeyCode::Char('o') => {
                self.prompt = Some(PromptState {
                    kind: PromptKind::OpenCapture,
                    input: self
                        .current_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_default(),
                })
            }
            KeyCode::Char('f') => {
                self.prompt = Some(PromptState {
                    kind: PromptKind::Filter,
                    input: self.filter.clone(),
                })
            }
            KeyCode::Char('s') => {
                if self.current_path.is_none() {
                    self.status = StatusMessage::error("Open or capture a session before saving.");
                } else {
                    self.prompt = Some(PromptState {
                        kind: PromptKind::SaveCapture,
                        input: self.suggest_save_path(),
                    });
                }
            }
            KeyCode::Char('i') => self.cycle_interface(1),
            KeyCode::Char('I') => self.cycle_interface(-1),
            KeyCode::Char('c') => self.toggle_capture(),
            KeyCode::Char('r') => {
                self.refresh_interfaces();
                self.refresh_visible_section();
                self.status = StatusMessage::info("Refreshed current section.");
            }
            _ => {}
        }

        Ok(true)
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) -> Result<bool, String> {
        let Some(prompt) = self.prompt.as_mut() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc => {
                self.prompt = None;
            }
            KeyCode::Enter => {
                let prompt = self.prompt.take().expect("prompt should exist");
                self.submit_prompt(prompt)?;
            }
            KeyCode::Backspace => {
                prompt.input.pop();
            }
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                prompt.input.push(character);
            }
            _ => {}
        }

        Ok(true)
    }

    fn submit_prompt(&mut self, prompt: PromptState) -> Result<(), String> {
        match prompt.kind {
            PromptKind::OpenCapture => {
                let value = prompt.input.trim();
                if value.is_empty() {
                    self.status = StatusMessage::error("Capture path cannot be empty.");
                    return Ok(());
                }
                self.open_capture(PathBuf::from(value));
            }
            PromptKind::Filter => {
                self.filter = prompt.input.trim().to_string();
                self.invalidate_cached_reports();
                self.refresh_visible_section();
                if self.filter.is_empty() {
                    self.status = StatusMessage::info("Packet filter cleared.");
                } else {
                    self.status = StatusMessage::info(format!("Applied filter: {}", self.filter));
                }
            }
            PromptKind::SaveCapture => {
                let output_path = prompt.input.trim();
                if output_path.is_empty() {
                    self.status = StatusMessage::error("Output path cannot be empty.");
                    return Ok(());
                }
                let Some(source_path) = self.current_path.clone() else {
                    self.status = StatusMessage::error("No capture is open.");
                    return Ok(());
                };
                let report = SaveCaptureService::default().save(SaveCaptureInput {
                    source_path,
                    output_path: PathBuf::from(output_path),
                    filter: self.current_filter(),
                    stream_filter: None,
                })?;
                self.status = StatusMessage::info(format!(
                    "Saved {} packets to {}.",
                    report.packets_written,
                    report.output_path.display()
                ));
            }
        }

        Ok(())
    }

    fn open_capture(&mut self, path: PathBuf) {
        if self.active_capture.is_some() {
            self.status =
                StatusMessage::error("Stop the current live capture before opening another file.");
            return;
        }

        match InspectCaptureService::default().inspect(InspectCaptureInput { path: path.clone() }) {
            Ok(report) => {
                self.current_path = Some(path);
                self.capture_summary = Some(report);
                self.packet_selection = 0;
                self.packet_list_state.select(Some(0));
                self.packet_detail_scroll = 0;
                self.invalidate_cached_reports();
                self.refresh_visible_section();
                self.status = StatusMessage::info("Capture loaded into the CLI app.");
            }
            Err(error) => {
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_interfaces(&mut self) {
        let previous_selection = self
            .available_interfaces
            .get(self.selected_interface)
            .cloned();
        let coordinator = LiveCaptureCoordinator::default();
        match coordinator.list_interfaces() {
            Ok(interfaces) => {
                self.available_interfaces = interfaces.into_iter().map(|item| item.name).collect();
                if self.available_interfaces.is_empty() {
                    self.available_interfaces.push("default".to_string());
                }
                self.selected_interface = previous_selection
                    .and_then(|selected| {
                        self.available_interfaces
                            .iter()
                            .position(|interface| interface == &selected)
                    })
                    .or_else(|| preferred_interface_index(&self.available_interfaces))
                    .unwrap_or(0);
            }
            Err(error) => {
                self.available_interfaces = vec!["default".to_string()];
                self.selected_interface = 0;
                self.status = StatusMessage::error(render_capture_error(error));
            }
        }
    }

    fn refresh_visible_section(&mut self) {
        self.refresh_capture_summary();

        match self.current_section() {
            Section::Packets => self.refresh_packets(),
            Section::Stats => self.refresh_stats(),
            Section::Conversations => self.refresh_conversations(),
            Section::Streams => self.refresh_streams(),
            Section::Transactions => self.refresh_transactions(),
            Section::Engine => {}
        }
    }

    fn refresh_capture_summary(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.capture_summary = None;
            return;
        };

        match InspectCaptureService::default().inspect(InspectCaptureInput { path }) {
            Ok(report) => self.capture_summary = Some(report),
            Err(error) => self.status = StatusMessage::error(error),
        }
    }

    fn refresh_packets(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.packet_list = None;
            self.packet_detail = None;
            return;
        };

        match ListPacketsService::default().list(ListPacketsInput {
            path: path.clone(),
            limit: None,
            filter: self.current_filter(),
        }) {
            Ok(report) => {
                self.packet_list = Some(report);
                let packet_count = self
                    .packet_list
                    .as_ref()
                    .map(|report| report.packets.len())
                    .unwrap_or(0);
                if packet_count == 0 {
                    self.packet_selection = 0;
                    self.packet_list_state.select(Some(0));
                    self.packet_detail = None;
                    self.packet_detail_scroll = 0;
                    return;
                }
                self.packet_selection = min(self.packet_selection, packet_count - 1);
                self.packet_list_state.select(Some(self.packet_selection));
                self.refresh_packet_detail();
            }
            Err(error) => {
                self.packet_list = None;
                self.packet_detail = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_packet_detail(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.packet_detail = None;
            return;
        };
        let Some(packet) = self
            .packet_list
            .as_ref()
            .and_then(|report| report.packets.get(self.packet_selection))
        else {
            self.packet_detail = None;
            return;
        };

        match InspectPacketService::default().inspect(InspectPacketInput {
            path,
            packet_index: packet.summary.index,
        }) {
            Ok(report) => {
                self.packet_detail = Some(report);
                self.packet_detail_scroll = 0;
            }
            Err(error) => {
                self.packet_detail = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_stats(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.stats_report = None;
            return;
        };

        match CaptureStatsService::default().stats(CaptureStatsInput {
            path,
            filter: self.current_filter(),
        }) {
            Ok(report) => self.stats_report = Some(report),
            Err(error) => {
                self.stats_report = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_conversations(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.conversations_report = None;
            return;
        };

        match ConversationsService::default().list(ConversationsInput {
            path,
            filter: self.current_filter(),
        }) {
            Ok(report) => self.conversations_report = Some(report),
            Err(error) => {
                self.conversations_report = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_streams(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.streams_report = None;
            return;
        };

        match StreamsService::default().list(StreamsInput {
            path,
            filter: self.current_filter(),
            stream_filter: None,
        }) {
            Ok(report) => self.streams_report = Some(report),
            Err(error) => {
                self.streams_report = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn refresh_transactions(&mut self) {
        let Some(path) = self.current_path.clone() else {
            self.transactions_report = None;
            return;
        };

        match TransactionsService::default().list(TransactionsInput {
            path,
            filter: self.current_filter(),
            transaction_filter: None,
        }) {
            Ok(report) => self.transactions_report = Some(report),
            Err(error) => {
                self.transactions_report = None;
                self.status = StatusMessage::error(error);
            }
        }
    }

    fn invalidate_cached_reports(&mut self) {
        self.packet_list = None;
        self.packet_detail = None;
        self.stats_report = None;
        self.conversations_report = None;
        self.streams_report = None;
        self.transactions_report = None;
    }

    fn next_section(&mut self) {
        self.section_index = (self.section_index + 1) % Section::ALL.len();
        self.refresh_visible_section();
    }

    fn previous_section(&mut self) {
        if self.section_index == 0 {
            self.section_index = Section::ALL.len() - 1;
        } else {
            self.section_index -= 1;
        }
        self.refresh_visible_section();
    }

    fn move_up(&mut self) {
        if self.current_section() == Section::Packets {
            if self.packet_selection > 0 {
                self.packet_selection -= 1;
                self.packet_list_state.select(Some(self.packet_selection));
                self.refresh_packet_detail();
            }
            return;
        }

        let scroll = self.current_scroll_mut();
        *scroll = scroll.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.current_section() == Section::Packets {
            let packet_count = self
                .packet_list
                .as_ref()
                .map(|report| report.packets.len())
                .unwrap_or(0);
            if packet_count > 0 && self.packet_selection + 1 < packet_count {
                self.packet_selection += 1;
                self.packet_list_state.select(Some(self.packet_selection));
                self.refresh_packet_detail();
            }
            return;
        }

        let scroll = self.current_scroll_mut();
        *scroll = scroll.saturating_add(1);
    }

    fn go_top(&mut self) {
        if self.current_section() == Section::Packets {
            self.packet_selection = 0;
            self.packet_list_state.select(Some(0));
            self.refresh_packet_detail();
        } else {
            *self.current_scroll_mut() = 0;
        }
    }

    fn go_bottom(&mut self) {
        if self.current_section() == Section::Packets {
            let packet_count = self
                .packet_list
                .as_ref()
                .map(|report| report.packets.len())
                .unwrap_or(0);
            if packet_count > 0 {
                self.packet_selection = packet_count - 1;
                self.packet_list_state.select(Some(self.packet_selection));
                self.refresh_packet_detail();
            }
        } else {
            *self.current_scroll_mut() = u16::MAX / 2;
        }
    }

    fn scroll_detail_up(&mut self) {
        if self.current_section() == Section::Packets {
            self.packet_detail_scroll = self.packet_detail_scroll.saturating_sub(3);
        } else {
            let scroll = self.current_scroll_mut();
            *scroll = scroll.saturating_sub(6);
        }
    }

    fn scroll_detail_down(&mut self) {
        if self.current_section() == Section::Packets {
            self.packet_detail_scroll = self.packet_detail_scroll.saturating_add(3);
        } else {
            let scroll = self.current_scroll_mut();
            *scroll = scroll.saturating_add(6);
        }
    }

    fn current_scroll_mut(&mut self) -> &mut u16 {
        match self.current_section() {
            Section::Stats => &mut self.stats_scroll,
            Section::Conversations => &mut self.conversations_scroll,
            Section::Streams => &mut self.streams_scroll,
            Section::Transactions => &mut self.transactions_scroll,
            Section::Engine => &mut self.engine_scroll,
            Section::Packets => &mut self.packet_detail_scroll,
        }
    }

    fn cycle_interface(&mut self, direction: i32) {
        if self.available_interfaces.is_empty() {
            return;
        }
        let len = self.available_interfaces.len() as i32;
        let next = (self.selected_interface as i32 + direction).rem_euclid(len);
        self.selected_interface = next as usize;
        self.status = StatusMessage::info(format!(
            "Interface selected: {}",
            self.current_interface_label()
        ));
    }

    fn toggle_capture(&mut self) {
        if self.active_capture.is_some() {
            let Some(session) = self.active_capture.take() else {
                return;
            };
            let stopped_path = match session.stop() {
                Ok(path) => path,
                Err(error) => {
                    self.status = StatusMessage::error(render_capture_error(error));
                    return;
                }
            };
            self.current_path = Some(stopped_path.clone());
            self.capture_state_label = "idle".to_string();
            self.invalidate_cached_reports();
            self.refresh_visible_section();
            self.status = StatusMessage::info(format!(
                "Live capture stopped and opened from {}.",
                stopped_path.display()
            ));
            return;
        }

        self.refresh_interfaces();
        let interface = self
            .available_interfaces
            .get(self.selected_interface)
            .and_then(|value| {
                if value == "default" {
                    None
                } else {
                    Some(value.clone())
                }
            });
        let coordinator = LiveCaptureCoordinator::default();
        let session = match coordinator.start(StartLiveCaptureInput { interface }) {
            Ok(session) => session,
            Err(error) => {
                self.status = StatusMessage::error(render_capture_error(error));
                return;
            }
        };
        let capture_path = session.path().to_path_buf();
        let capture_interface = session.interface().to_string();
        self.current_path = Some(capture_path);
        self.active_capture = Some(session);
        self.capture_state_label = "running".to_string();
        self.invalidate_cached_reports();
        self.refresh_visible_section();
        self.last_live_refresh = Instant::now();
        self.status =
            StatusMessage::info(format!("Live capture started on {}.", capture_interface));
    }

    fn on_tick(&mut self) {
        let Some(session) = self.active_capture.as_mut() else {
            return;
        };

        match session.is_running() {
            Ok(true) => {
                self.capture_state_label = "running".to_string();
                if self.last_live_refresh.elapsed() >= LIVE_REFRESH_INTERVAL {
                    self.refresh_capture_summary();
                    if self.current_section() == Section::Packets {
                        self.refresh_packets();
                    }
                    self.last_live_refresh = Instant::now();
                }
            }
            Ok(false) => {
                self.capture_state_label = "exited".to_string();
            }
            Err(error) => {
                self.capture_state_label = "error".to_string();
                self.status = StatusMessage::error(render_capture_error(error));
            }
        }
    }

    fn shutdown(&mut self) {
        if let Some(session) = self.active_capture.take() {
            let _ = session.stop();
        }
    }

    fn current_capture_label(&self) -> String {
        self.current_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn filter_label(&self) -> String {
        if self.filter.trim().is_empty() {
            "none".to_string()
        } else {
            self.filter.trim().to_string()
        }
    }

    fn current_interface_label(&self) -> String {
        self.available_interfaces
            .get(self.selected_interface)
            .cloned()
            .unwrap_or_else(|| "default".to_string())
    }

    fn suggest_save_path(&self) -> String {
        let Some(path) = self.current_path.as_ref() else {
            return "capture-filtered.pcap".to_string();
        };
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("capture");
        parent
            .join(format!("{stem}-filtered.pcap"))
            .display()
            .to_string()
    }

    fn stats_text(&self) -> String {
        self.stats_report
            .as_ref()
            .map(render_capture_stats_report)
            .unwrap_or_else(|| "No stats loaded.".to_string())
    }

    fn conversations_text(&self) -> String {
        self.conversations_report
            .as_ref()
            .map(render_conversation_report)
            .unwrap_or_else(|| "No conversation data loaded.".to_string())
    }

    fn streams_text(&self) -> String {
        self.streams_report
            .as_ref()
            .map(render_stream_report)
            .unwrap_or_else(|| "No stream data loaded.".to_string())
    }

    fn transactions_text(&self) -> String {
        self.transactions_report
            .as_ref()
            .map(render_transaction_report)
            .unwrap_or_else(|| "No transaction data loaded.".to_string())
    }

    fn packet_detail_text(&self) -> String {
        self.packet_detail
            .as_ref()
            .map(render_packet_detail_report)
            .unwrap_or_else(|| {
                "No packet selected.\n\nMove through the list with j/k or arrow keys.".to_string()
            })
    }
}

fn preferred_interface_index(interfaces: &[String]) -> Option<usize> {
    for preferred in ["en0", "eth0", "wlan0", "wlp0s20f3", "wlp2s0"] {
        if let Some(index) = interfaces
            .iter()
            .position(|interface| interface == preferred)
        {
            return Some(index);
        }
    }

    interfaces.iter().position(|interface| {
        !matches!(
            interface.as_str(),
            "any" | "lo" | "lo0" | "Loopback" | "Npcap Loopback Adapter"
        )
    })
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn truncate_text(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn section_hint(section: Section) -> &'static str {
    match section {
        Section::Packets => "live capture and packet drill-down",
        Section::Stats => "packet and protocol totals",
        Section::Conversations => "bidirectional flow summaries",
        Section::Streams => "stream and TLS session analysis",
        Section::Transactions => "HTTP and TLS transaction rows",
        Section::Engine => "capabilities and runtime profile",
    }
}
