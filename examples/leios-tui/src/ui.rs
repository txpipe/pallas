//! Renders the [`Dashboard`] as a ratatui screen.

use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::dashboard::{ChainView, Dashboard, EbRow, EbStage};

/// Draws the whole dashboard for the current frame.
pub fn draw(f: &mut Frame, d: &Dashboard, table_state: &mut TableState) {
    let rows = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(8), // praos + overlay
        Constraint::Min(6),    // eb table
        Constraint::Length(8), // log
        Constraint::Length(1), // footer
    ])
    .split(f.area());

    render_header(f, d, rows[0]);

    let mid =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);
    render_praos(f, d, mid[0]);
    render_overlay(f, d, mid[1]);

    render_ebs(f, d, table_state, rows[2]);
    render_log(f, d, rows[3]);
    render_footer(f, d, rows[4]);
}

fn render_header(f: &mut Frame, d: &Dashboard, area: Rect) {
    let (dot, status) = if let Some(p) = &d.peer {
        let leios = if p.leios { "Leios ✓" } else { "Leios ✗" };
        (
            Span::styled("●", Style::default().fg(Color::Green)),
            format!("magic {}   N2N v{}   {}", d.magic, p.version, leios),
        )
    } else {
        (
            Span::styled("○", Style::default().fg(Color::DarkGray)),
            format!("magic {}   connecting…", d.magic),
        )
    };

    let addr = d
        .peer
        .as_ref()
        .map(|p| p.addr.clone())
        .unwrap_or_else(|| d.relay.clone());

    let line = Line::from(vec![
        Span::raw(format!("peer {addr}   ")),
        dot,
        Span::raw(format!(
            "   {status}   uptime {}",
            fmt_dur(d.started.elapsed())
        )),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Leios testnet · initiator ");
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn render_praos(f: &mut Frame, d: &Dashboard, area: Rect) {
    let c = &d.chain;
    let lag = match (c.tip_height, c.local_height) {
        (Some(t), Some(l)) => format!("{} blocks", t.saturating_sub(l)),
        _ => "—".to_string(),
    };
    let rate = hdr_rate(c, Duration::from_secs(30));
    let bars = spark(&hdr_buckets(c, 16, Duration::from_secs(60)));
    let era = c.era.map(era_name).unwrap_or_else(|| "—".to_string());

    let lines = vec![
        Line::from(format!("tip    {}", fmt_pt(c.tip_height, c.tip_slot))),
        Line::from(format!("local  {}", fmt_pt(c.local_height, c.local_slot))),
        Line::from(format!("lag    {lag}")),
        Line::from(format!("era    {era}")),
        Line::from(format!("hdr/s  {rate:.1}  {bars}")),
        Line::from(format!(
            "       headers {}   rollbacks {}",
            c.headers, c.rollbacks
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Praos chain ");
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_overlay(f: &mut Frame, d: &Dashboard, area: Rect) {
    let o = &d.overlay;
    let lines = vec![
        Line::from(format!("announced   {}", group(o.announced))),
        Line::from(format!(
            "offered     {}    bodies  {}",
            group(o.offered),
            group(o.bodies)
        )),
        Line::from(format!(
            "txs offered {}    txs     {} / {} tx",
            group(o.txs_offered),
            group(o.txs_ebs),
            group(o.tx_count)
        )),
        Line::from(format!(
            "votes       {}    voters  {}",
            group(o.votes),
            group(o.voters.len() as u64)
        )),
        Line::from(format!("fetched     {}", human_bytes(o.bytes))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Leios overlay ");
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_ebs(f: &mut Frame, d: &Dashboard, table_state: &mut TableState, area: Rect) {
    let rows: Vec<Row> = d
        .ebs
        .values()
        .rev()
        .map(|r| {
            let hash = if r.hash.len() >= 4 {
                format!("{}…", hex::encode(&r.hash[..4]))
            } else {
                hex::encode(&r.hash)
            };
            let size = r
                .size
                .map(|s| human_bytes(s as u64))
                .unwrap_or_else(|| "—".to_string());
            let txs = match r.tx_total {
                Some(n) => format!("{}/{}", r.tx_fetched, n),
                None => "—".to_string(),
            };
            let (votes, vstyle) = if r.votes > 0 {
                (format!("{} ●", r.votes), Style::default().fg(Color::Green))
            } else {
                ("—".to_string(), Style::default().fg(Color::DarkGray))
            };

            Row::new(vec![
                Cell::from(group(r.slot)),
                Cell::from(hash),
                Cell::from(size),
                Cell::from(txs),
                Cell::from(votes).style(vstyle),
                Cell::from(lifecycle(r)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(7),
        Constraint::Min(22),
    ];
    let header = Row::new(vec!["slot", "eb hash", "size", "txs", "votes", "lifecycle"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Endorser Blocks  ({}) ", d.ebs.len())),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▌");

    let len = d.ebs.len();
    if len == 0 {
        table_state.select(None);
    } else {
        let sel = if d.follow { 0 } else { d.selected.min(len - 1) };
        table_state.select(Some(sel));
    }

    f.render_stateful_widget(table, area, table_state);
}

fn render_log(f: &mut Frame, d: &Dashboard, area: Rect) {
    let height = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = match d.log.lock() {
        Ok(buf) => buf
            .iter()
            .rev()
            .take(height)
            .rev()
            .map(|s| Line::from(s.clone()))
            .collect(),
        Err(_) => Vec::new(),
    };

    let block = Block::default().borders(Borders::ALL).title(" Log ");
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_footer(f: &mut Frame, d: &Dashboard, area: Rect) {
    let follow = if d.follow { "on" } else { "off" };
    let line = Line::from(format!(
        " q quit    ↑/↓ scroll    f follow:{follow}    c clear log "
    ))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(line), area);
}

/// Builds the lifecycle chip for an EB row: reached stages bright, pending dim,
/// with a short hint naming the next awaited step.
fn lifecycle(row: &EbRow) -> Line<'static> {
    let reached = |on: bool, label: &str| {
        let style = if on {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Span::styled(label.to_string(), style)
    };
    let sep = || Span::styled("▸".to_string(), Style::default().fg(Color::DarkGray));

    let body = row.stage >= EbStage::BodyFetched;
    let txs = row.stage >= EbStage::TxsFetched;
    let voted = row.votes > 0;

    let mut spans = vec![
        reached(true, "offer"),
        sep(),
        reached(body, "body"),
        sep(),
        reached(txs, "txs"),
        sep(),
        reached(voted, "vote"),
    ];

    let hint = if !body {
        "  awaiting body"
    } else if row.stage < EbStage::TxsOffered {
        "  awaiting offer"
    } else if !txs {
        "  txs offered"
    } else {
        ""
    };
    if !hint.is_empty() {
        spans.push(Span::styled(
            hint.to_string(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
    }

    Line::from(spans)
}

// ---------------------------------------------------------------------------
// formatting helpers
// ---------------------------------------------------------------------------

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

fn fmt_pt(height: Option<u64>, slot: Option<u64>) -> String {
    match (height, slot) {
        (Some(h), Some(s)) => format!("#{} · slot {}", group(h), group(s)),
        (None, Some(s)) => format!("#? · slot {}", group(s)),
        _ => "—".to_string(),
    }
}

/// Inserts thin spaces every three digits for readability.
fn group(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

fn era_name(variant: u8) -> String {
    let name = match variant {
        0 => "Byron",
        1 => "Shelley",
        2 => "Allegra",
        3 => "Mary",
        4 => "Alonzo",
        5 => "Babbage",
        6 => "Conway",
        7 => "Dijkstra",
        _ => return format!("era {variant}"),
    };
    format!("{name} ({variant})")
}

/// Header arrivals per bucket over `window`, oldest-left / newest-right.
fn hdr_buckets(chain: &ChainView, n: usize, window: Duration) -> Vec<u64> {
    let now = Instant::now();
    let bucket = window.as_secs_f64() / n as f64;
    let mut out = vec![0u64; n];
    for &t in &chain.hdr_times {
        let age = now.saturating_duration_since(t).as_secs_f64();
        if age >= window.as_secs_f64() {
            continue;
        }
        let idx = ((age / bucket) as usize).min(n - 1);
        out[n - 1 - idx] += 1;
    }
    out
}

fn hdr_rate(chain: &ChainView, window: Duration) -> f64 {
    let now = Instant::now();
    let count = chain
        .hdr_times
        .iter()
        .filter(|&&t| now.saturating_duration_since(t) < window)
        .count();
    count as f64 / window.as_secs_f64()
}

fn spark(data: &[u64]) -> String {
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = data.iter().copied().max().unwrap_or(0);
    if max == 0 {
        return BARS[0].to_string().repeat(data.len());
    }
    data.iter()
        .map(|&v| {
            let idx = ((v as f64 / max as f64) * (BARS.len() - 1) as f64).round() as usize;
            BARS[idx.min(BARS.len() - 1)]
        })
        .collect()
}
