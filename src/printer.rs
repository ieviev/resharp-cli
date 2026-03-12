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
        }
    }
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

pub fn write_results(
    mut out: &mut dyn WriteColor,
    buf: &[u8],
    matches: &[LineMatch],
    path: Option<&str>,
    opts: &PrinterOpts,
) -> anyhow::Result<()> {

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

    // build full line index for context
    let line_starts = build_line_starts(buf);
    let total_lines = line_starts.len();
    let has_context = opts.before_ctx > 0 || opts.after_ctx > 0;

    let match_lines: std::collections::HashSet<usize> =
        matches.iter().map(|m| m.line_number).collect();
    let mut last_printed_line: Option<usize> = None;

    for lm in matches {
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
