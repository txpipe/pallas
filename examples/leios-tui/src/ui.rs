//! Renders the [`Dashboard`] as a ratatui screen — an educational view of the
//! Leios overlay.
//!
//! Ranking blocks (RBs) and endorser blocks (EBs) are two **vertically-aligned
//! swim lanes** sharing one column axis: the newest `N` RBs define the columns
//! (newest hugging the tip on the right), and each EB is drawn in the column of
//! the RB it belongs to. That shared column *is* the connection — an EB sits
//! directly under its RB. The window slides as the tip advances; boxes fill in
//! place. The EB→RB association is a slot heuristic (see [`EbRow::column_rb`]).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::dashboard::{ChainView, Dashboard, EbRow, EbStage, RbCard};

/// Target column box width and inter-column gap, shared by both lanes so their
/// columns line up. Wide enough to fit labelled content; fewer columns fit at
/// once, which is fine — the window follows the tip.
const BOX_W: u16 = 26;
const GAP: u16 = 1;

/// Height of an RB box (border + hash line + slot line).
const RB_BOX_H: u16 = 4;

/// Height of an EB card (border + 9 content lines: slot, txs label, tx bar,
/// votes label, vote bar, then one row per lifecycle phase).
const EB_BOX_H: u16 = 11;

/// Draws the whole dashboard for the current frame.
pub fn draw(f: &mut Frame, d: &Dashboard) {
    let rows = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(7), // RB lane (boxes + rate line)
        Constraint::Min(14),   // EB lane (tall boxes + detail strip)
        Constraint::Length(5), // log
        Constraint::Length(1), // footer
    ])
    .split(f.area());

    render_header(f, d, rows[0]);
    render_lanes(f, d, rows[1], rows[2]);
    render_log(f, d, rows[3]);
    render_footer(f, d, rows[4]);
}

fn render_header(f: &mut Frame, d: &Dashboard, area: Rect) {
    let (dot, status) = if let Some(p) = &d.peer {
        let leios = if p.leios { "Leios ✓" } else { "Leios ✗" };
        let era = d.chain.era.map(era_name).unwrap_or_else(|| "—".to_string());
        (
            Span::styled("●", Style::default().fg(Color::Green)),
            format!(
                "connected   magic {}   N2N v{}   {}   era {}",
                d.magic, p.version, leios, era
            ),
        )
    } else {
        (
            Span::styled("○", Style::default().fg(Color::DarkGray)),
            format!("connecting…   magic {}", d.magic),
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
        Span::raw(format!("   {status}")),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Leios · Musashi Dojo ")
        .title(Line::from(format!("uptime {} ", fmt_dur(d.started.elapsed()))).right_aligned());
    f.render_widget(Paragraph::new(line).block(block), area);
}

// ---------------------------------------------------------------------------
// Aligned swim lanes: columns are the newest N RBs; EBs render in their RB's
// column. `rb_area` and `eb_area` are full-width rows, so their inner areas
// share x/width — hence identical column x-positions.
// ---------------------------------------------------------------------------

/// The RB(s) visible in the columns, plus where each RB `block_no` maps.
struct ColumnPlan<'a> {
    cols: usize,
    /// `columns[i]` = the RB drawn in column `i` (0 = left, `cols-1` = tip), if any.
    columns: Vec<Option<&'a RbCard>>,
    /// RB `block_no` → column index, for placing EBs.
    col_of: HashMap<u64, usize>,
}

fn plan_columns(chain: &ChainView, inner_width: u16) -> ColumnPlan<'_> {
    let cols = (((inner_width + GAP) / (BOX_W + GAP)).max(1)) as usize;
    let mut columns: Vec<Option<&RbCard>> = vec![None; cols];
    let mut col_of = HashMap::new();
    // Newest RB → rightmost (tip) column; older ones fill leftward.
    for (k, rb) in chain.rbs.iter().rev().take(cols).enumerate() {
        let col = cols - 1 - k;
        columns[col] = Some(rb);
        col_of.insert(rb.block_no, col);
    }
    ColumnPlan {
        cols,
        columns,
        col_of,
    }
}

/// Column x-origin for column `i` within `inner`.
fn column_x(inner: Rect, i: usize) -> u16 {
    inner.x + i as u16 * (BOX_W + GAP)
}

fn render_lanes(f: &mut Frame, d: &Dashboard, rb_area: Rect, eb_area: Rect) {
    // Column plan from the (shared) inner width.
    let inner_width = rb_area.width.saturating_sub(2);
    let plan = plan_columns(&d.chain, inner_width);
    let tip_col = plan.cols.saturating_sub(1);

    // EBs newest-first; resolve the selection and the selected EB's column.
    let ebs: Vec<&EbRow> = d.ebs.values().rev().collect();
    let sel = if ebs.is_empty() {
        None
    } else if d.follow {
        Some(0)
    } else {
        Some(d.selected.min(ebs.len() - 1))
    };
    let selected_col = sel
        .and_then(|i| ebs.get(i))
        .and_then(|row| eb_column(row, &plan.col_of, tip_col));

    render_rb_lane(f, d, rb_area, &plan, selected_col);
    render_eb_lane(f, d, eb_area, &plan, &ebs, sel, selected_col);
}

/// Which column an EB belongs to: its RB's column, or the tip column while
/// pending. `None` if its RB has scrolled out of the visible window.
fn eb_column(row: &EbRow, col_of: &HashMap<u64, usize>, tip_col: usize) -> Option<usize> {
    match row.column_rb {
        Some(bn) => col_of.get(&bn).copied(),
        None => Some(tip_col),
    }
}

fn render_rb_lane(
    f: &mut Frame,
    d: &Dashboard,
    area: Rect,
    plan: &ColumnPlan,
    selected_col: Option<usize>,
) {
    let c = &d.chain;
    let tip = format!(" tip {} ", fmt_pt(c.tip_height, c.tip_slot));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Ranking blocks — Praos chain ")
        .title(Line::from(tip).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let split = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let boxes = split[0];

    if c.rbs.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from("  waiting for blocks…"))
                .style(Style::default().fg(Color::DarkGray)),
            boxes,
        );
    } else {
        let h = boxes.height.min(RB_BOX_H);
        for (i, slot) in plan.columns.iter().enumerate() {
            let Some(rb) = slot else { continue };
            let x = column_x(boxes, i);
            if x + BOX_W > boxes.x + boxes.width {
                break;
            }
            let selected = selected_col == Some(i);
            let border = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                era_color(rb.era)
            };
            let b = Block::default()
                .borders(Borders::ALL)
                .border_style(border)
                .title(Line::from(format!("#{}", rb.block_no)));
            let rect = Rect {
                x,
                y: boxes.y,
                width: BOX_W,
                height: h,
            };
            let content = vec![
                Line::from(short_hash(&rb.hash, 8)),
                Line::from(Span::styled(
                    format!("slot {}", group(rb.slot)),
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            f.render_widget(Paragraph::new(content).block(b), rect);
        }
    }

    // Rate / rollback readout beneath the RB boxes.
    let rate = hdr_rate(c, Duration::from_secs(30));
    let bars = spark(&hdr_buckets(c, 16, Duration::from_secs(60)));
    let line = Line::from(format!(
        " headers {}   rollbacks {}   {:.1} hdr/s  {}",
        group(c.headers),
        group(c.rollbacks),
        rate,
        bars
    ))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(line), split[1]);
}

fn render_eb_lane(
    f: &mut Frame,
    d: &Dashboard,
    area: Rect,
    plan: &ColumnPlan,
    ebs: &[&EbRow],
    sel: Option<usize>,
    selected_col: Option<usize>,
) {
    let o = &d.overlay;
    let totals = format!(
        " {} seen · {} txs · {} votes ",
        group(d.ebs.len() as u64),
        group(o.tx_count),
        group(o.votes)
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Endorser blocks — Leios overlay ")
        .title(Line::from(totals).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let split = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let boxes = split[0];
    let detail_area = split[1];

    if ebs.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from("  waiting for endorser blocks…"))
                .style(Style::default().fg(Color::DarkGray)),
            boxes,
        );
        return;
    }

    let tip_col = plan.cols.saturating_sub(1);
    let scale = d.vote_scale();

    // Per column: the EB to display (newest mapping there, unless the selected EB
    // is in this column) and how many EBs map there (for a `+k` badge).
    let mut display: Vec<Option<usize>> = vec![None; plan.cols];
    let mut count: Vec<usize> = vec![0; plan.cols];
    for (i, row) in ebs.iter().enumerate() {
        if let Some(col) = eb_column(row, &plan.col_of, tip_col) {
            if display[col].is_none() {
                display[col] = Some(i); // newest-first ⇒ first seen is newest
            }
            count[col] += 1;
        }
    }
    if let (Some(s), Some(col)) = (sel, selected_col) {
        display[col] = Some(s);
    }

    let h = boxes.height.min(EB_BOX_H);
    for (col, slot) in display.iter().enumerate() {
        let Some(i) = slot else { continue };
        let x = column_x(boxes, col);
        if x + BOX_W > boxes.x + boxes.width {
            break;
        }
        let rect = Rect {
            x,
            y: boxes.y,
            width: BOX_W,
            height: h,
        };
        render_eb_box(
            f,
            ebs[*i],
            sel == Some(*i),
            count[col].saturating_sub(1),
            scale,
            rect,
        );
    }

    // Detail strip for the selected EB.
    if let Some(row) = sel.and_then(|i| ebs.get(i)) {
        f.render_widget(Paragraph::new(eb_detail(row, &plan.col_of)), detail_area);
    }
}

fn render_eb_box(
    f: &mut Frame,
    row: &EbRow,
    selected: bool,
    extra: usize,
    scale: usize,
    area: Rect,
) {
    let badge = if extra > 0 {
        format!(" +{extra}")
    } else {
        String::new()
    };
    let border = if selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Line::from(format!(
            "EB {}{badge}",
            short_hash(&row.hash, 4)
        )));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Each metric is a `label value` row followed by a full-width bar beneath…
    let bar_w = inner.width as usize;
    let tx_ratio = match row.tx_total {
        Some(0) => 1.0,
        Some(t) => row.tx_fetched as f64 / t as f64,
        None => 0.0,
    };
    let tx_val = match row.tx_total {
        Some(t) => format!("{}/{}", row.tx_fetched, t),
        None => format!("{}/?", row.tx_fetched),
    };

    let mut lines = vec![
        Line::from(format!("slot {}", group(row.slot))),
        Line::from(format!("txs {tx_val}")),
        full_bar(tx_ratio, bar_w, Color::Cyan),
        Line::from(format!("votes {}/{}", row.voters.len(), scale)),
        full_bar(row.vote_ratio(scale), bar_w, Color::Green),
    ];
    // …then one checklist row per lifecycle phase.
    for (glyph, label, color) in phase_rows(row) {
        lines.push(Line::from(Span::styled(
            format!("{glyph} {label}"),
            Style::default().fg(color),
        )));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// The EB's lifecycle phases as `(glyph, label, colour)` rows: `✓` done (green),
/// `◐` in progress (yellow), `○` pending (grey). Rendered as a checklist inside
/// each EB box.
fn phase_rows(row: &EbRow) -> [(&'static str, &'static str, Color); 4] {
    let done = ("✓", Color::Green);
    let active = ("◐", Color::Yellow);
    let pending = ("○", Color::DarkGray);

    // Offered is implied by the box existing. Body is fetched automatically after
    // the offer, so it's "active" until it lands.
    let body = if row.stage >= EbStage::BodyFetched {
        done
    } else {
        active
    };
    let txs = match row.tx_total {
        Some(0) => done,
        Some(t) if row.tx_fetched >= t => done,
        _ if row.tx_fetched > 0 || row.stage >= EbStage::TxsOffered => active,
        _ => pending,
    };
    let votes = if row.votes > 0 { done } else { pending };

    [
        (done.0, "offered", done.1),
        (body.0, "body", body.1),
        (txs.0, "download txs", txs.1),
        (votes.0, "votes", votes.1),
    ]
}

/// A full-width progress bar (`████░░░░`) — the label and value sit on the row
/// above it (see [`render_eb_box`]).
fn full_bar(ratio: f64, width: usize, color: Color) -> Line<'static> {
    let width = width.max(1);
    let filled = ((ratio.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    let mut bar = String::with_capacity(width);
    for _ in 0..filled {
        bar.push('█');
    }
    for _ in filled..width {
        bar.push('░');
    }
    Line::from(Span::styled(bar, Style::default().fg(color)))
}

/// One-line full readout of the selected EB.
fn eb_detail(row: &EbRow, col_of: &HashMap<u64, usize>) -> Line<'static> {
    let hash = if row.hash.len() >= 6 {
        format!("{}…", hex::encode(&row.hash[..6]))
    } else {
        hex::encode(&row.hash)
    };
    let size = row
        .size
        .map(|s| human_bytes(s as u64))
        .unwrap_or_else(|| "—".to_string());
    let txs = match row.tx_total {
        Some(t) => format!("{}/{}", row.tx_fetched, t),
        None => format!("{}/?", row.tx_fetched),
    };
    let rb = match row.column_rb {
        Some(bn) if col_of.contains_key(&bn) => format!("RB #{bn}"),
        Some(bn) => format!("RB #{bn} (off-window)"),
        None => "RB pending".to_string(),
    };
    let (_, stage, color) = stage_glyph(row);
    Line::from(vec![
        Span::raw(format!(
            " selected  EB {hash} · slot {} · {size} · txs {txs} · votes {} · ",
            group(row.slot),
            row.voters.len()
        )),
        Span::styled(stage.to_string(), Style::default().fg(color)),
        Span::raw(format!(" · {rb}")),
    ])
}

/// The EB's lifecycle stage as a `(glyph, label, colour)` triple. `voting`
/// (any votes seen) takes precedence over the fetch stage.
fn stage_glyph(row: &EbRow) -> (&'static str, &'static str, Color) {
    if row.votes > 0 {
        return ("●", "voting", Color::Green);
    }
    match row.stage {
        EbStage::Offered => ("○", "offered", Color::DarkGray),
        EbStage::BodyFetched | EbStage::TxsOffered => ("◐", "downloading", Color::Yellow),
        EbStage::TxsFetched => ("◑", "txs complete", Color::Cyan),
    }
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
        " q quit   ←/→ select EB   f follow:{follow}   c clear log    · column = nearest RB by slot (heuristic) · vote bar ∝ peak voters"
    ))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(Paragraph::new(line), area);
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

/// First `nbytes` of a hash as hex with an ellipsis (`—` if empty).
fn short_hash(hash: &[u8], nbytes: usize) -> String {
    if hash.is_empty() {
        return "—".to_string();
    }
    format!("{}…", hex::encode(&hash[..nbytes.min(hash.len())]))
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

/// Border tint for an RB box, keyed to its era.
fn era_color(variant: u8) -> Style {
    let color = match variant {
        7 => Color::Magenta, // Dijkstra
        6 => Color::Blue,    // Conway
        5 => Color::Cyan,    // Babbage
        _ => Color::DarkGray,
    };
    Style::default().fg(color)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::{EbRow, EbStage, RbCard};
    use pallas_network2::protocol::Point;
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::HashSet;

    fn draw_at(d: &Dashboard, w: u16, h: u16) {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, d)).unwrap();
    }

    /// A dashboard with RBs and EBs linked to columns (plus one pending EB), so
    /// rendering exercises the aligned-lane paths.
    fn populated() -> Dashboard {
        let mut d = Dashboard::new("relay:3001".into(), 164, crate::logbuf::new_log());
        d.chain.tip_height = Some(12_043);
        d.chain.tip_slot = Some(2_814_902);
        d.chain.era = Some(7);
        for i in 0..20u64 {
            d.chain.rbs.push_back(RbCard {
                block_no: 12_000 + i,
                slot: 2_814_000 + i * 10,
                era: 7,
                hash: vec![i as u8; 32],
            });
        }
        for i in 0..8u64 {
            let hash = vec![i as u8; 32];
            let voters: HashSet<u64> = (0..i).collect();
            // Link most EBs to a recent RB; leave the last one pending.
            let column_rb = if i < 7 { Some(12_012 + i) } else { None };
            d.ebs.insert(
                Point::Specific(2_814_120 + i * 10, hash.clone()),
                EbRow {
                    slot: 2_814_120 + i * 10,
                    hash,
                    size: Some(8_192),
                    tx_total: Some(64),
                    tx_fetched: (i * 8) as usize,
                    votes: i as usize,
                    voters,
                    stage: EbStage::TxsFetched,
                    column_rb,
                },
            );
        }
        d.peak_voters = 7;
        d
    }

    #[test]
    fn draw_never_panics_across_sizes_and_states() {
        let sizes = [(80, 24), (120, 40), (40, 12), (200, 60), (30, 8)];

        let empty = Dashboard::new("relay:3001".into(), 164, crate::logbuf::new_log());
        for (w, h) in sizes {
            draw_at(&empty, w, h);
        }

        let full = populated();
        for (w, h) in sizes {
            draw_at(&full, w, h);
        }

        // Follow off with a selection past the end (clamped) and a narrow width
        // that forces few columns.
        let mut scrolled = populated();
        scrolled.follow = false;
        scrolled.selected = 999;
        draw_at(&scrolled, 100, 30);
        draw_at(&scrolled, 28, 20);
    }
}
