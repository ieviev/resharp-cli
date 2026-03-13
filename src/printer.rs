use std::collections::HashSet;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::args::Args;
use crate::search::LineMatch;

pub struct PrinterOpts {
    pub heading: bool,
    pub line_number: bool,
    pub only_matching: bool,
    pub count: bool,
    pub files_with_matches: bool,
    pub files_without_match: bool,
    pub column: bool,
    pub byte_offset: bool,
    pub before_ctx: usize,
    pub after_ctx: usize,
    pub replace: Option<String>,
    pub json: bool,
    pub unique: bool,
    pub show_scope: bool,
}

impl PrinterOpts {
    pub fn from_args(args: &Args) -> Self {
        let multi = args.paths.len() > 1
            || args.paths.first().map_or(false, |p| p.is_dir())
            || args.paths.is_empty(); // default to "." which is a dir

        Self {
            heading: args.show_heading(),
            line_number: args.show_line_number(multi),
            only_matching: args.only_matching,
            count: args.count,
            files_with_matches: args.files_with_matches,
            files_without_match: args.files_without_match,
            column: args.column,
            byte_offset: args.byte_offset,
            before_ctx: args.before_ctx(),
            after_ctx: args.after_ctx(),
            replace: args.replace.clone(),
            json: args.json,
            unique: args.unique,
            show_scope: args.show_scope,
        }
    }
}

#[derive(serde::Serialize)]
struct JsonMatchLine {
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    line_number: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    byte_offset: Option<usize>,
    lines: String,
    submatches: Vec<JsonSubmatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

#[derive(serde::Serialize)]
struct JsonSubmatch {
    #[serde(rename = "match")]
    match_text: String,
    start: usize,
    end: usize,
}

pub fn print_results(
    buf: &[u8],
    matches: &[LineMatch],
    path: Option<&str>,
    opts: &PrinterOpts,
    color_choice: ColorChoice,
) -> anyhow::Result<()> {
    let mut out = StandardStream::stdout(color_choice);
    write_results(&mut out, buf, matches, path, opts)
}

/// thread-local unique set for dedup (caller manages lifetime)
pub struct UniqueSet {
    seen: HashSet<Vec<u8>>,
}

impl UniqueSet {
    pub fn new() -> Self {
        Self { seen: HashSet::new() }
    }

    fn check(&mut self, key: &[u8]) -> bool {
        if self.seen.contains(key) {
            false
        } else {
            self.seen.insert(key.to_vec());
            true
        }
    }
}

pub fn write_results(
    out: &mut dyn WriteColor,
    buf: &[u8],
    matches: &[LineMatch],
    path: Option<&str>,
    opts: &PrinterOpts,
) -> anyhow::Result<()> {
    write_results_with_unique(out, buf, matches, path, opts, None)
}

pub fn write_results_with_unique(
    mut out: &mut dyn WriteColor,
    buf: &[u8],
    matches: &[LineMatch],
    path: Option<&str>,
    opts: &PrinterOpts,
    unique_set: Option<&mut UniqueSet>,
) -> anyhow::Result<()> {
    // create local unique set if needed but none provided
    let mut local_set = if opts.unique && unique_set.is_none() {
        Some(UniqueSet::new())
    } else {
        None
    };
    let mut unique_set = match unique_set {
        Some(us) => Some(us),
        None => local_set.as_mut(),
    };

    if opts.json {
        return write_json_results(out, buf, matches, path, opts, unique_set);
    }

    // files-without-match mode
    if opts.files_without_match {
        if matches.is_empty() {
            if let Some(p) = path {
                writeln!(out, "{p}")?;
            }
        }
        return Ok(());
    }

    // files-with-matches mode
    if opts.files_with_matches {
        if !matches.is_empty() {
            if let Some(p) = path {
                out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                write!(out, "{p}")?;
                out.reset()?;
                writeln!(out)?;
            }
        }
        return Ok(());
    }

    // count mode
    if opts.count {
        if matches.is_empty() {
            return Ok(());
        }
        if let Some(p) = path {
            out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
            write!(out, "{p}")?;
            out.reset()?;
            write!(out, ":")?;
        }
        writeln!(out, "{}", matches.len())?;
        return Ok(());
    }

    if matches.is_empty() {
        return Ok(());
    }

    // heading mode: print path once
    if opts.heading {
        if let Some(p) = path {
            out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
            writeln!(out, "{p}")?;
            out.reset()?;
        }
    }

    // build full line index for context and show-scope
    let line_starts = build_line_starts(buf);
    let total_lines = line_starts.len();
    let has_context = opts.before_ctx > 0 || opts.after_ctx > 0;

    let match_lines: std::collections::HashSet<usize> =
        matches.iter().map(|m| m.line_number).collect();
    let mut last_printed_line: Option<usize> = None;

    // show-scope: track last printed scope to avoid repeating
    let mut last_scope_line: Option<usize> = None;

    for lm in matches {
        // unique check
        if opts.unique {
            let line = get_line(buf, &line_starts, lm.line_number);
            let key = if opts.only_matching && !lm.match_ranges.is_empty() {
                let (ms, me) = lm.match_ranges[0];
                &line[ms.min(line.len())..me.min(line.len())]
            } else {
                line
            };
            if let Some(ref mut us) = unique_set.as_deref_mut() {
                if !us.check(key) {
                    continue;
                }
            }
        }

        // show-scope: print enclosing scope marker
        if opts.show_scope {
            if let Some((scope_line, scope_text)) = find_enclosing_scope(buf, &line_starts, lm.line_number) {
                if last_scope_line != Some(scope_line) {
                    last_scope_line = Some(scope_line);
                    // print scope header
                    if !opts.heading {
                        if let Some(p) = path {
                            out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                            write!(out, "{p}")?;
                            out.reset()?;
                            write!(out, ":")?;
                        }
                    }
                    out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
                    write!(out, "{}:", scope_line + 1)?;
                    out.reset()?;
                    out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_italic(true))?;
                    writeln!(out, "  {scope_text}")?;
                    out.reset()?;
                }
            }
        }

        let ctx_start = lm.line_number.saturating_sub(opts.before_ctx);
        let ctx_end = (lm.line_number + opts.after_ctx).min(total_lines.saturating_sub(1));

        // separator between non-adjacent groups
        if has_context {
            if let Some(last) = last_printed_line {
                if ctx_start > last + 1 {
                    writeln!(out, "--")?;
                }
            }
        }

        if has_context {
            // print before-context
            for line_idx in ctx_start..lm.line_number {
                if last_printed_line.map_or(true, |l| line_idx > l) {
                    print_context_line(
                        &mut out, buf, &line_starts, line_idx, path, opts,
                    )?;
                    last_printed_line = Some(line_idx);
                }
            }
        }

        // print the match line
        if opts.only_matching {
            let line = get_line(buf, &line_starts, lm.line_number);
            for &(ms, me) in &lm.match_ranges {
                print_prefix(&mut out, path, opts, lm, false)?;
                out.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                if let Some(ref repl) = opts.replace {
                    out.write_all(repl.as_bytes())?;
                } else {
                    out.write_all(&line[ms..me])?;
                }
                out.reset()?;
                writeln!(out)?;
            }
        } else {
            print_match_line(&mut out, buf, &line_starts, lm, path, opts)?;
        }
        last_printed_line = Some(lm.line_number);

        if has_context {
            // print after-context, skipping lines that are match lines
            for line_idx in (lm.line_number + 1)..=ctx_end {
                if !match_lines.contains(&line_idx) {
                    print_context_line(
                        &mut out, buf, &line_starts, line_idx, path, opts,
                    )?;
                }
                last_printed_line = Some(line_idx);
            }
        }
    }

    Ok(())
}

fn write_json_results(
    out: &mut dyn WriteColor,
    buf: &[u8],
    matches: &[LineMatch],
    path: Option<&str>,
    opts: &PrinterOpts,
    mut unique_set: Option<&mut UniqueSet>,
) -> anyhow::Result<()> {
    if opts.files_without_match {
        if matches.is_empty() {
            if let Some(p) = path {
                let obj = serde_json::json!({"type": "file", "path": p});
                writeln!(out, "{}", serde_json::to_string(&obj)?)?;
            }
        }
        return Ok(());
    }

    if opts.files_with_matches {
        if !matches.is_empty() {
            if let Some(p) = path {
                let obj = serde_json::json!({"type": "file", "path": p});
                writeln!(out, "{}", serde_json::to_string(&obj)?)?;
            }
        }
        return Ok(());
    }

    if opts.count {
        if !matches.is_empty() {
            let obj = serde_json::json!({
                "type": "count",
                "path": path,
                "count": matches.len(),
            });
            writeln!(out, "{}", serde_json::to_string(&obj)?)?;
        }
        return Ok(());
    }

    let line_starts = build_line_starts(buf);

    for lm in matches {
        let line = get_line(buf, &line_starts, lm.line_number);
        let line_text = String::from_utf8_lossy(line).into_owned();

        if opts.unique {
            let key = if opts.only_matching && !lm.match_ranges.is_empty() {
                let (ms, me) = lm.match_ranges[0];
                &line[ms.min(line.len())..me.min(line.len())]
            } else {
                line
            };
            if let Some(ref mut us) = unique_set {
                if !us.check(key) {
                    continue;
                }
            }
        }

        let submatches: Vec<JsonSubmatch> = lm.match_ranges.iter().map(|&(ms, me)| {
            let ms = ms.min(line.len());
            let me = me.min(line.len());
            JsonSubmatch {
                match_text: String::from_utf8_lossy(&line[ms..me]).into_owned(),
                start: ms,
                end: me,
            }
        }).collect();

        let column = lm.match_ranges.first().map(|&(ms, _)| ms + 1);
        let byte_offset = if opts.byte_offset { Some(lm.line_start) } else { None };

        let scope = if opts.show_scope {
            find_enclosing_scope(buf, &line_starts, lm.line_number)
                .map(|(_, text)| text)
        } else {
            None
        };

        let obj = JsonMatchLine {
            path: path.map(|s| s.to_string()),
            line_number: lm.line_number + 1,
            column,
            byte_offset,
            lines: line_text,
            submatches,
            scope,
        };

        writeln!(out, "{}", serde_json::to_string(&obj)?)?;
    }

    Ok(())
}

fn build_line_starts(buf: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' && i + 1 < buf.len() {
            starts.push(i + 1);
        }
    }
    starts
}

fn get_line<'a>(buf: &'a [u8], line_starts: &[usize], line_idx: usize) -> &'a [u8] {
    let start = line_starts[line_idx];
    let end = if line_idx + 1 < line_starts.len() {
        line_starts[line_idx + 1]
    } else {
        buf.len()
    };
    let line = &buf[start..end];
    // strip trailing \n / \r\n
    if line.ends_with(b"\r\n") {
        &line[..line.len() - 2]
    } else if line.ends_with(b"\n") {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn get_indent(buf: &[u8], line_starts: &[usize], line_idx: usize) -> usize {
    let line = get_line(buf, line_starts, line_idx);
    line.iter().take_while(|&&b| b == b' ' || b == b'\t').count()
}

fn find_enclosing_scope(buf: &[u8], line_starts: &[usize], match_line: usize) -> Option<(usize, String)> {
    let match_indent = get_indent(buf, line_starts, match_line);

    for line_idx in (0..match_line).rev() {
        let line = get_line(buf, line_starts, line_idx);

        // skip blank lines
        if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
            continue;
        }

        let indent = line.iter().take_while(|&&b| b == b' ' || b == b'\t').count();

        if indent < match_indent {
            let text = std::str::from_utf8(line).unwrap_or("");
            let trimmed = text.trim();
            if looks_like_scope_marker(trimmed) {
                return Some((line_idx, trimmed.to_string()));
            }
        }
    }
    None
}

fn looks_like_scope_marker(line: &str) -> bool {
    let markers = [
        "fn ", "async fn ", "pub fn ", "pub async fn ", "pub(crate) fn ",
        "pub(super) fn ", "pub(crate) async fn ", "unsafe fn ", "pub unsafe fn ",
        "def ", "async def ",
        "class ", "pub struct ", "struct ", "pub enum ", "enum ",
        "impl ", "impl<", "pub trait ", "trait ",
        "func ", "function ", "method ",
        "pub mod ", "mod ",
        "interface ", "pub type ", "type ",
        "export function ", "export async function ", "export default function ",
        "export class ", "export interface ", "export type ",
    ];
    markers.iter().any(|m| line.starts_with(m))
}

fn print_prefix(
    out: &mut dyn WriteColor,
    path: Option<&str>,
    opts: &PrinterOpts,
    lm: &LineMatch,
    is_context: bool,
) -> anyhow::Result<()> {
    let sep = if is_context { "-" } else { ":" };

    if !opts.heading {
        if let Some(p) = path {
            out.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
            write!(out, "{p}")?;
            out.reset()?;
            write!(out, "{sep}")?;
        }
    }

    if opts.line_number {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(out, "{}", lm.line_number + 1)?;
        out.reset()?;
        write!(out, "{sep}")?;
    }

    if opts.column && !lm.match_ranges.is_empty() {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(out, "{}", lm.match_ranges[0].0 + 1)?;
        out.reset()?;
        write!(out, "{sep}")?;
    }

    if opts.byte_offset {
        out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(out, "{}", lm.line_start)?;
        out.reset()?;
        write!(out, "{sep}")?;
    }

    Ok(())
}

fn print_match_line(
    out: &mut dyn WriteColor,
    buf: &[u8],
    line_starts: &[usize],
    lm: &LineMatch,
    path: Option<&str>,
    opts: &PrinterOpts,
) -> anyhow::Result<()> {
    print_prefix(out, path, opts, lm, false)?;

    let line = get_line(buf, line_starts, lm.line_number);

    if lm.match_ranges.is_empty() {
        // inverted match, no highlights
        out.write_all(line)?;
        writeln!(out)?;
        return Ok(());
    }

    // print line with highlighted matches (or replacements)
    let mut pos = 0;
    for &(ms, me) in &lm.match_ranges {
        let ms = ms.min(line.len());
        let me = me.min(line.len());
        if ms > pos {
            out.write_all(&line[pos..ms])?;
        }
        out.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        if let Some(ref repl) = opts.replace {
            out.write_all(repl.as_bytes())?;
        } else {
            out.write_all(&line[ms..me])?;
        }
        out.reset()?;
        pos = me;
    }
    if pos < line.len() {
        out.write_all(&line[pos..])?;
    }
    writeln!(out)?;
    Ok(())
}

fn print_context_line(
    out: &mut dyn WriteColor,
    buf: &[u8],
    line_starts: &[usize],
    line_idx: usize,
    path: Option<&str>,
    opts: &PrinterOpts,
) -> anyhow::Result<()> {
    let dummy = LineMatch {
        line_number: line_idx,
        line_start: line_starts[line_idx],
        match_ranges: vec![],
    };
    print_prefix(out, path, opts, &dummy, true)?;
    let line = get_line(buf, line_starts, line_idx);
    out.write_all(line)?;
    writeln!(out)?;
    Ok(())
}
