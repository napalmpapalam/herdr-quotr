//! Print every theme, rendered through the real paint stack: `cargo run --example themes`.
//!
//! Pass `html` to emit one page holding all of them, for comparing them side by side.

use std::{env, fmt::Write as _, path::Path};

use anyhow::{Result, anyhow};
use markup::Pos;
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
};
use transcript::Transcript;

/// Wide enough for the 100-column measure plus margins, tall enough for the whole fixture.
const SIZE: (u16, u16) = (118, 34);

fn main() -> Result<()> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/fixture.jsonl");
    let transcript = Transcript::load(&fixture)?;
    let html = env::args().nth(1).is_some_and(|arg| arg == "html");

    if html {
        println!("{HTML_HEAD}");
    }
    for name in ui::THEMES {
        let theme = ui::theme::resolve(name).ok_or_else(|| anyhow!("unknown theme {name}"))?;
        let buffer = frame(&transcript, &theme)?;
        if html {
            println!("<section>\n<h2>{name}</h2>\n<pre>{}</pre>\n</section>", markup_html(&buffer));
            continue;
        }
        println!("\n\x1b[1m── {name} ─────────────────────────────────────────────\x1b[0m");
        print!("{}", ansi(&buffer));
    }
    if html {
        println!("{HTML_TAIL}");
        return Ok(());
    }
    println!("\n{} themes.", ui::THEMES.len());
    Ok(())
}

/// One rendered frame. Selects a range and banks a pair, so the fill, the gutter, and a card
/// are all on screen.
fn frame(transcript: &Transcript, theme: &ui::Theme) -> Result<Buffer> {
    let lines: Vec<ui::SourceLine<'_>> = transcript
        .lines()
        .iter()
        .map(|line| ui::SourceLine { text: &line.text, tone: line.tone })
        .collect();
    let markup: Vec<ui::Markup<'_>> = transcript
        .lines()
        .iter()
        .map(|line| ui::Markup {
            text: &line.text,
            tone: line.tone,
            block: line.block,
            spans: &line.spans,
        })
        .collect();
    let styles = ui::analyze(&markup, theme, ui::DEFAULT_MEASURE);

    let banked = [ui::Banked {
        number: 1,
        from: 3,
        to: 4,
        question: "why the union and not just the selection?",
        quote: transcript.lines().get(3).map_or("", |line| line.text.as_str()),
    }];
    let view = ui::View {
        lines: &lines,
        styles: &styles,
        palette: theme.palette,
        measure: ui::DEFAULT_MEASURE,
        turns: transcript.turn_starts(),
        cursor: Pos { line: 8, col: 12 },
        selection: Some((Pos { line: 7, col: 4 }, Pos { line: 8, col: 40 })),
        banked: &banked,
        scroll: ui::Scroll::From(0),
        question: None,
        status: "38 lines",
    };

    let mut terminal = Terminal::new(TestBackend::new(SIZE.0, SIZE.1))?;
    terminal.draw(|f| {
        ui::render(f, &view);
    })?;
    Ok(terminal.backend().buffer().clone())
}

/// The page shell. quotr paints no background of its own — its colors sit on whatever the
/// terminal's is — so the ground is a control, not a constant.
const HTML_HEAD: &str = include_str!("themes.head.html");
const HTML_TAIL: &str = "</main>";

/// The rendered buffer as HTML, one `<span>` per run of like-styled cells.
fn markup_html(buffer: &Buffer) -> String {
    let area = buffer.area();
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        let (mut run, mut style) = (String::new(), String::new());
        for x in area.left()..area.right() {
            let Some(cell) = buffer.cell((x, y)) else { continue };
            let next = css(cell.style());
            if next != style && !run.is_empty() {
                let _ = write!(out, "<span style=\"{style}\">{run}</span>");
                run.clear();
            }
            style = next;
            run.push_str(match cell.symbol() {
                "<" => "&lt;",
                ">" => "&gt;",
                "&" => "&amp;",
                other => other,
            });
        }
        let _ = writeln!(out, "<span style=\"{style}\">{run}</span>");
    }
    out
}

fn css(style: ratatui::style::Style) -> String {
    let mut rules: Vec<String> = Vec::new();
    rules.extend(style.fg.and_then(hex).map(|c| format!("color:{c}")));
    rules.extend(style.bg.and_then(hex).map(|c| format!("background:{c}")));
    for (modifier, rule) in [
        (Modifier::BOLD, "font-weight:700"),
        (Modifier::DIM, "opacity:.7"),
        (Modifier::ITALIC, "font-style:italic"),
        (Modifier::UNDERLINED, "text-decoration:underline"),
        (Modifier::CROSSED_OUT, "text-decoration:line-through"),
    ] {
        if style.add_modifier.contains(modifier) {
            rules.push(rule.to_owned());
        }
    }
    rules.join(";")
}

fn hex(color: Color) -> Option<String> {
    match color {
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        _ => None,
    }
}

/// The rendered buffer as ANSI escapes, so a terminal shows what the popup would.
fn ansi(buffer: &Buffer) -> String {
    let area = buffer.area();
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let Some(cell) = buffer.cell((x, y)) else { continue };
            let _ = write!(out, "\x1b[0m{}{}", sgr(cell.style()), cell.symbol());
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

fn sgr(style: ratatui::style::Style) -> String {
    let mut codes: Vec<String> = Vec::new();
    codes.extend(style.fg.and_then(|fg| color(fg, 38)));
    codes.extend(style.bg.and_then(|bg| color(bg, 48)));
    for (modifier, code) in [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::CROSSED_OUT, "9"),
    ] {
        if style.add_modifier.contains(modifier) {
            codes.push(code.to_owned());
        }
    }
    if codes.is_empty() { String::new() } else { format!("\x1b[{}m", codes.join(";")) }
}

/// `base` is 38 for a foreground, 48 for a background.
fn color(color: Color, base: u8) -> Option<String> {
    match color {
        Color::Rgb(r, g, b) => Some(format!("{base};2;{r};{g};{b}")),
        Color::Indexed(i) => Some(format!("{base};5;{i}")),
        _ => None,
    }
}
