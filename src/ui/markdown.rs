//! Markdown rendering for assistant messages, drawn with plain egui widgets.
//!
//! Milestone 1 renders paragraphs, headings, bullet and numbered lists,
//! blockquotes and fenced code blocks. Richer inline styling arrives later.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Layout, RichText, Sense, Stroke, Ui};

use super::theme;

/// One parsed block of markdown.
#[derive(Debug, Clone, PartialEq)]
enum Block {
    /// Plain paragraph text.
    Paragraph(String),
    /// Heading with its level.
    Heading(u8, String),
    /// Bullet list items.
    List(Vec<String>),
    /// Numbered list items.
    OrderedList(Vec<String>),
    /// Quote text.
    Quote(String),
    /// Code block with optional language.
    Code(Option<String>, String),
}

/// Render `markdown` into `ui`.
pub fn show(ui: &mut Ui, markdown: &str) {
    for block in parse(markdown) {
        match block {
            Block::Paragraph(text) => paragraph(ui, &text),
            Block::Heading(level, text) => draw_heading(ui, level, &text),
            Block::List(items) => list(ui, &items, false),
            Block::OrderedList(items) => list(ui, &items, true),
            Block::Quote(text) => quote(ui, &text),
            Block::Code(lang, code) => code_block(ui, lang.as_deref(), &code),
        }
    }
}

fn paragraph(ui: &mut Ui, text: &str) {
    let color = ui.visuals().text_color();
    ui.add(egui::Label::new(inline_job(ui, text, color)).wrap());
    ui.add_space(theme::SPACE_4);
}

/// Style one paragraph or list item: `code` chips, **bold**, *italic* and [links](url).
fn inline_job(ui: &Ui, text: &str, color: Color32) -> LayoutJob {
    let visuals = ui.visuals();
    let strong = visuals.strong_text_color();
    let code_bg = visuals.code_bg_color;
    let accent = visuals.hyperlink_color;
    let body = theme::body_font();
    let mono = FontId::monospace(theme::TEXT_BODY - 1.5);
    let line_height = theme::line_height(theme::TEXT_BODY);
    let mut job = LayoutJob::default();
    let (mut bold, mut italic) = (false, false);
    let mut plain = String::new();
    let chars: Vec<char> = text.chars().collect();
    let flush = |job: &mut LayoutJob, plain: &mut String, bold: bool, italic: bool| {
        if plain.is_empty() {
            return;
        }
        job.append(
            plain,
            0.0,
            TextFormat {
                font_id: body.clone(),
                color: if bold { strong } else { color },
                italics: italic,
                line_height,
                ..Default::default()
            },
        );
        plain.clear();
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|x| *x == '`') {
                flush(&mut job, &mut plain, bold, italic);
                let code: String = chars[i + 1..i + 1 + end].iter().collect();
                job.append(
                    &format!(" {code} "),
                    0.0,
                    TextFormat {
                        font_id: mono.clone(),
                        color,
                        background: code_bg,
                        line_height,
                        ..Default::default()
                    },
                );
                i += end + 2;
                continue;
            }
        } else if c == '*' && chars.get(i + 1) == Some(&'*') {
            flush(&mut job, &mut plain, bold, italic);
            bold = !bold;
            i += 2;
            continue;
        } else if (c == '*' || c == '_')
            && !chars.get(i + 1).is_none_or(|n| n.is_whitespace())
            && (italic || chars[i + 1..].contains(&c))
            && (i == 0 || !chars[i - 1].is_alphanumeric())
        {
            flush(&mut job, &mut plain, bold, italic);
            italic = !italic;
            i += 1;
            continue;
        } else if c == '[' {
            if let Some(close) = chars[i + 1..].iter().position(|x| *x == ']') {
                let after = i + 1 + close + 1;
                if chars.get(after) == Some(&'(') {
                    if let Some(end) = chars[after + 1..].iter().position(|x| *x == ')') {
                        flush(&mut job, &mut plain, bold, italic);
                        let label: String = chars[i + 1..i + 1 + close].iter().collect();
                        job.append(
                            &label,
                            0.0,
                            TextFormat {
                                font_id: body.clone(),
                                color: accent,
                                underline: Stroke::new(1.0_f32, accent),
                                line_height,
                                ..Default::default()
                            },
                        );
                        i = after + 1 + end + 1;
                        continue;
                    }
                }
            }
        }
        plain.push(c);
        i += 1;
    }
    flush(&mut job, &mut plain, bold, italic);
    job
}

fn draw_heading(ui: &mut Ui, level: u8, text: &str) {
    let size = match level {
        1 => theme::TEXT_TITLE + 4.0,
        2 => theme::TEXT_TITLE,
        _ => theme::TEXT_TITLE - 2.0,
    };
    ui.add_space(theme::SPACE_4);
    ui.label(RichText::new(text).strong().size(size));
    ui.add_space(theme::SPACE_4);
}

fn list(ui: &mut Ui, items: &[String], ordered: bool) {
    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", i + 1)
        } else {
            "• ".to_owned()
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(marker)
                    .font(theme::muted_font())
                    .color(ui.visuals().weak_text_color()),
            );
            let color = ui.visuals().text_color();
            ui.add(egui::Label::new(inline_job(ui, item, color)).wrap());
        });
    }
    ui.add_space(theme::SPACE_4);
}

fn quote(ui: &mut Ui, text: &str) {
    let bar = ui.visuals().widgets.noninteractive.bg_stroke.color;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(3.0, theme::line_height(theme::TEXT_BODY).unwrap_or(theme::TEXT_BODY)),
            Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, theme::chip_radius(), bar);
        let color = ui.visuals().weak_text_color();
        ui.add(egui::Label::new(inline_job(ui, text, color)).wrap());
    });
    ui.add_space(theme::SPACE_4);
}

/// A rounded code box with a language label and copy button.
pub fn code_block(ui: &mut Ui, lang: Option<&str>, code: &str) {
    let bg = ui.visuals().code_bg_color;
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(theme::card_radius())
        .inner_margin(theme::card_margin())
        .show(ui, |ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                copy_button(ui, code);
                ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                    if let Some(lang) = lang {
                        ui.label(
                            RichText::new(lang.to_lowercase())
                                .font(theme::caption_font())
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                });
            });
            ui.add_space(theme::SPACE_4);
            ui.add(
                egui::Label::new(
                    RichText::new(code)
                        .font(theme::mono_font())
                        .color(ui.visuals().text_color()),
                )
                .wrap(),
            );
        });
    ui.add_space(theme::SPACE_8);
}

/// A small copy button; returns true on the click that copies.
pub fn copy_button(ui: &mut Ui, text: &str) -> bool {
    let btn = super::icons::button(ui, super::icons::Icon::Copy, ui.visuals().weak_text_color());
    if btn.clicked() {
        ui.ctx().copy_text(text.to_owned());
        true
    } else {
        false
    }
}

/// The parsed blocks of `markdown`, with their kind names. Test helper.
#[cfg(test)]
pub fn parse_for_test(markdown: &str) -> Vec<ParsedBlock> {
    parse(markdown)
        .into_iter()
        .map(|b| ParsedBlock {
            kind_name: match b {
                Block::Paragraph(_) => "paragraph",
                Block::Heading(..) => "heading",
                Block::List(_) => "list",
                Block::OrderedList(_) => "ordered",
                Block::Quote(_) => "quote",
                Block::Code(..) => "code",
            }
            .to_owned(),
        })
        .collect()
}

/// One parsed block, named. Test helper.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBlock {
    /// The block's kind name.
    pub kind_name: String,
}

/// Split markdown into blocks. Good enough for chat answers.
fn parse(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut list: Option<(bool, Vec<String>)> = None;
    let mut quote: Vec<&str> = Vec::new();
    let mut code: Option<(Option<String>, Vec<&str>)> = None;

    macro_rules! flush_paragraph {
        () => {
            if !paragraph.is_empty() {
                blocks.push(Block::Paragraph(paragraph.join("\n")));
                paragraph.clear();
            }
        };
    }
    macro_rules! flush_list {
        () => {
            if let Some((ordered, items)) = list.take() {
                blocks.push(if ordered {
                    Block::OrderedList(items)
                } else {
                    Block::List(items)
                });
            }
        };
    }

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if let Some(lang) = fence_open(trimmed) {
            if code.is_none() {
                flush_paragraph!();
                flush_list!();
                code = Some((lang, Vec::new()));
                continue;
            }
            if let Some((lang, lines)) = code.take() {
                blocks.push(Block::Code(lang, lines.join("\n")));
            }
            continue;
        }
        if let Some((_, lines)) = code.as_mut() {
            lines.push(line);
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph!();
            flush_list!();
            if !quote.is_empty() {
                blocks.push(Block::Quote(quote.join("\n")));
                quote.clear();
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("> ") {
            flush_paragraph!();
            flush_list!();
            quote.push(rest);
            continue;
        }
        if trimmed == ">" {
            quote.push("");
            continue;
        }
        if !quote.is_empty() {
            blocks.push(Block::Quote(quote.join("\n")));
            quote.clear();
        }
        if let Some((level, rest)) = heading(trimmed) {
            flush_paragraph!();
            flush_list!();
            blocks.push(Block::Heading(level, rest.to_owned()));
            continue;
        }
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph!();
            list.get_or_insert_with(|| (false, Vec::new()))
                .1
                .push(rest.to_owned());
            continue;
        }
        if let Some(rest) = numbered(trimmed) {
            flush_paragraph!();
            list.get_or_insert_with(|| (true, Vec::new()))
                .1
                .push(rest.to_owned());
            continue;
        }
        flush_list!();
        paragraph.push(line);
    }

    flush_paragraph!();
    flush_list!();
    if !quote.is_empty() {
        blocks.push(Block::Quote(quote.join("\n")));
    }
    if let Some((lang, lines)) = code {
        blocks.push(Block::Code(lang, lines.join("\n")));
    }
    blocks
}

/// `` ```lang `` opens a block, bare `` ``` `` closes one.
fn fence_open(line: &str) -> Option<Option<String>> {
    let rest = line.strip_prefix("```")?;
    if rest.is_empty() {
        Some(None)
    } else {
        Some(Some(rest.trim().to_owned()))
    }
}

/// `## Title` returns `(2, "Title")`.
fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].trim_start();
    Some((hashes as u8, rest))
}

/// `1. item` returns `"item"`.
fn numbered(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    line.get(digits..)?.strip_prefix(". ")
}
