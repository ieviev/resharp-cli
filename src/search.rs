use std::io::Read;
use std::path::Path;

use crate::args::Args;
use crate::printer::{self, PrinterOpts};

/// a single line match with metadata
pub struct LineMatch {
    pub line_number: usize,
    pub line_start: usize,
    pub match_ranges: Vec<(usize, usize)>,
}

/// search result for a single file
pub struct SearchResult {
    pub matches: Vec<LineMatch>,
    pub had_error: bool,
}

/// build line index: byte offset of each line start
fn build_line_index(buf: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, &b) in buf.iter().enumerate() {
        if b == b'\n' && i + 1 < buf.len() {
            starts.push(i + 1);
        }
    }
    starts
}

/// find which line a byte offset falls in (binary search)
fn offset_to_line(line_starts: &[usize], offset: usize) -> usize {
    match line_starts.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    }
}

/// find the end of a line (position of \n or end of buffer)
fn line_end(buf: &[u8], line_start: usize) -> usize {
    let rest = &buf[line_start..];
    match memchr::memchr(b'\n', rest) {
        Some(pos) => line_start + pos,
        None => buf.len(),
    }
}

/// search a byte buffer using find_all on the whole buffer
pub fn search_buffer(
    re: &resharp::Regex,
    buf: &[u8],
    args: &Args,
) -> SearchResult {
    if buf.is_empty() {
        return SearchResult {
            matches: vec![],
            had_error: false,
        };
    }

    let all_matches = match re.find_all(buf) {
        Ok(m) => m,
        Err(resharp::Error::CapacityExceeded) => {
            return SearchResult {
                matches: vec![],
                had_error: true,
            };
        }
        Err(_) => {
            return SearchResult {
                matches: vec![],
                had_error: true,
            };
        }
    };

    let line_starts = build_line_index(buf);
    let mut line_matches: Vec<LineMatch> = Vec::new();

    for m in &all_matches {
        if m.start == m.end && m.start == buf.len() {
            continue; // skip end-of-input empty match
        }
        let start_line = offset_to_line(&line_starts, m.start);
        let end_line = if m.end > m.start {
            offset_to_line(&line_starts, m.end.saturating_sub(1))
        } else {
            start_line
        };

        // emit each line spanned by this match
        for line_idx in start_line..=end_line {
            let ls = line_starts[line_idx];
            let le = line_end(buf, ls);
            let line_len = le - ls;

            // compute match range relative to this line
            let rel_start = if line_idx == start_line { m.start - ls } else { 0 };
            let rel_end = if line_idx == end_line {
                (m.end - ls).min(line_len)
            } else {
                line_len
            };
            let match_range = (rel_start, rel_end);

            if let Some(last) = line_matches.last_mut() {
                if last.line_number == line_idx {
                    last.match_ranges.push(match_range);
                    continue;
                }
            }

            line_matches.push(LineMatch {
                line_number: line_idx,
                line_start: ls,
                match_ranges: vec![match_range],
            });
        }
    }

    if args.invert_match {
        let matched_lines: std::collections::HashSet<usize> =
            line_matches.iter().map(|m| m.line_number).collect();
        let mut inverted = Vec::new();
        for (i, &ls) in line_starts.iter().enumerate() {
            if !matched_lines.contains(&i) {
                inverted.push(LineMatch {
                    line_number: i,
                    line_start: ls,
                    match_ranges: vec![],
                });
            }
        }
        return SearchResult {
            matches: inverted,
            had_error: false,
        };
    }

    if let Some(max) = args.max_count {
        line_matches.truncate(max);
    }

    SearchResult {
        matches: line_matches,
        had_error: false,
    }
}


/// read file contents, using mmap for large files
fn read_file(path: &Path, args: &Args) -> anyhow::Result<FileData> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    let len = metadata.len();

    if len == 0 {
        return Ok(FileData::Vec(vec![]));
    }

    if args.use_mmap(len) {
        // SAFETY: file is opened read-only, we don't modify the mapping
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(FileData::Mmap(mmap))
    } else {
        Ok(FileData::Vec(std::fs::read(path)?))
    }
}

enum FileData {
    Vec(Vec<u8>),
    Mmap(memmap2::Mmap),
}

impl AsRef<[u8]> for FileData {
    fn as_ref(&self) -> &[u8] {
        match self {
            FileData::Vec(v) => v,
            FileData::Mmap(m) => m,
        }
    }
}

/// detect binary files by checking for NUL bytes in the first 8KB
fn is_binary(buf: &[u8]) -> bool {
    let check_len = buf.len().min(8192);
    memchr::memchr(0, &buf[..check_len]).is_some()
}

/// search a file, print results to stdout
pub fn search_file(
    re: &resharp::Regex,
    path: &Path,
    args: &Args,
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
) -> anyhow::Result<(bool, bool)> {
    let data = read_file(path, args)?;
    let buf = data.as_ref();

    if !args.search_binary() && is_binary(buf) {
        return Ok((false, false));
    }

    let result = search_buffer(re, buf, args);

    if result.had_error {
        eprintln!("resharp: {}: DFA capacity exceeded, skipping", path.display());
        return Ok((false, true));
    }

    let found = !result.matches.is_empty();

    if args.quiet {
        return Ok((found, false));
    }

    let path_str = Some(path.to_string_lossy().into_owned());
    printer::print_results(buf, &result.matches, path_str.as_deref(), printer_opts, color_choice)?;

    Ok((found, false))
}

/// search a file, write results to a WriteColor buffer
pub fn search_file_to_writer(
    re: &resharp::Regex,
    path: &Path,
    args: &Args,
    printer_opts: &PrinterOpts,
    out: &mut dyn termcolor::WriteColor,
) -> anyhow::Result<(bool, bool)> {
    let data = read_file(path, args)?;
    let buf = data.as_ref();

    if !args.search_binary() && is_binary(buf) {
        return Ok((false, false));
    }

    let result = search_buffer(re, buf, args);

    if result.had_error {
        eprintln!("resharp: {}: DFA capacity exceeded, skipping", path.display());
        return Ok((false, true));
    }

    let found = !result.matches.is_empty();

    if args.quiet {
        return Ok((found, false));
    }

    let path_str = Some(path.to_string_lossy().into_owned());
    printer::write_results(out, buf, &result.matches, path_str.as_deref(), printer_opts)?;

    Ok((found, false))
}

/// search stdin
pub fn search_stdin(
    re: &resharp::Regex,
    args: &Args,
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
) -> anyhow::Result<bool> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    let result = search_buffer(re, &buf, args);

    if result.had_error {
        anyhow::bail!("DFA capacity exceeded");
    }

    let found = !result.matches.is_empty();

    if !args.quiet {
        printer::print_results(&buf, &result.matches, None, printer_opts, color_choice)?;
    }

    Ok(found)
}
