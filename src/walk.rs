use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ignore::overrides::OverrideBuilder;
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use termcolor::BufferWriter;

use crate::args::Args;
use crate::printer::PrinterOpts;
use crate::search;

pub fn print_type_list() {
    let mut builder = TypesBuilder::new();
    builder.add_defaults();
    let types = builder.build().unwrap();
    for def in types.definitions() {
        let globs = def.globs().join(", ");
        println!("{}: {globs}", def.name());
    }
}

/// returns (found_any, had_errors)
pub fn walk_and_search(
    re: &resharp::Regex,
    pattern: &str,
    args: &Args,
    paths: &[PathBuf],
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
) -> anyhow::Result<(bool, bool)> {
    let max_filesize = args.parse_max_filesize()?;

    if args.sort.as_deref() == Some("path") {
        return walk_sorted(re, args, paths, printer_opts, color_choice, max_filesize);
    }

    let threads = args.threads.unwrap_or(0);
    let use_parallel = match args.threads {
        Some(n) => n > 1,
        None => std::thread::available_parallelism().map_or(false, |n| n.get() > 1),
    };
    let walker = build_walker(args, paths, threads)?;

    if use_parallel {
        walk_parallel(walker, pattern, args, printer_opts, color_choice, max_filesize)
    } else {
        walk_sequential(walker, re, args, printer_opts, color_choice, max_filesize)
    }
}

fn build_walker(
    args: &Args,
    paths: &[PathBuf],
    threads: usize,
) -> anyhow::Result<WalkBuilder> {
    let mut builder = WalkBuilder::new(&paths[0]);
    for p in &paths[1..] {
        builder.add(p);
    }

    builder.hidden(!args.effective_hidden());
    if args.effective_no_ignore() {
        builder.ignore(false);
        builder.git_ignore(false);
        builder.git_global(false);
        builder.git_exclude(false);
    }
    if args.no_ignore_vcs {
        builder.git_ignore(false);
        builder.git_global(false);
        builder.git_exclude(false);
    }
    builder.follow_links(args.follow);
    if let Some(depth) = args.max_depth {
        builder.max_depth(Some(depth));
    }
    if threads > 0 {
        builder.threads(threads);
    }

    if !args.file_type.is_empty() || !args.type_not.is_empty() {
        let mut types = TypesBuilder::new();
        types.add_defaults();
        for t in &args.file_type {
            types.select(t);
        }
        for t in &args.type_not {
            types.negate(t);
        }
        builder.types(types.build()?);
    }

    if !args.glob.is_empty() || !args.iglob.is_empty() {
        let mut overrides = OverrideBuilder::new(".");
        for g in &args.glob {
            overrides.add(g)?;
        }
        for g in &args.iglob {
            overrides.case_insensitive(true)?.add(g)?;
        }
        builder.overrides(overrides.build()?);
    }

    Ok(builder)
}

fn walk_sequential(
    walker: WalkBuilder,
    re: &resharp::Regex,
    args: &Args,
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
    max_filesize: Option<u64>,
) -> anyhow::Result<(bool, bool)> {
    let mut found_any = false;
    let mut had_errors = false;

    for entry in walker.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("resharp: {err}");
                continue;
            }
        };

        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        if let Some(max) = max_filesize {
            if entry.metadata().map_or(false, |m| m.len() > max) {
                continue;
            }
        }

        let (found, had_error) =
            search::search_file(re, entry.path(), args, printer_opts, color_choice)?;

        if had_error {
            had_errors = true;
        }
        if found {
            found_any = true;
            if args.quiet {
                return Ok((true, had_errors));
            }
        }
    }

    Ok((found_any, had_errors))
}

fn walk_parallel(
    walker: WalkBuilder,
    pattern: &str,
    args: &Args,
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
    max_filesize: Option<u64>,
) -> anyhow::Result<(bool, bool)> {
    let bufwtr = Arc::new(BufferWriter::stdout(color_choice));
    let found_any = Arc::new(AtomicBool::new(false));
    let had_errors = Arc::new(AtomicBool::new(false));
    let quiet = args.quiet;
    let pattern = pattern.to_string();
    let dfa_threshold = args.dfa_threshold;
    let dfa_capacity = args.dfa_capacity;

    walker.build_parallel().run(|| {
        let found_any = Arc::clone(&found_any);
        let had_errors = Arc::clone(&had_errors);
        let bufwtr = Arc::clone(&bufwtr);
        let re = match resharp::Regex::with_options(&pattern, resharp::EngineOptions {
            dfa_threshold,
            max_dfa_capacity: dfa_capacity,
            ..Default::default()
        }) {
            Ok(re) => re,
            Err(e) => {
                eprintln!("resharp: failed to compile pattern: {e}");
                return Box::new(move |_| ignore::WalkState::Quit);
            }
        };
        Box::new(move |entry| {
            if quiet && found_any.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("resharp: {err}");
                    return ignore::WalkState::Continue;
                }
            };

            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                return ignore::WalkState::Continue;
            }

            if let Some(max) = max_filesize {
                if entry.metadata().map_or(false, |m| m.len() > max) {
                    return ignore::WalkState::Continue;
                }
            }

            let mut buf = bufwtr.buffer();
            match search::search_file_to_writer(&re, entry.path(), args, printer_opts, &mut buf) {
                Ok((found, had_error)) => {
                    if had_error {
                        had_errors.store(true, Ordering::Relaxed);
                    }
                    if found {
                        found_any.store(true, Ordering::Relaxed);
                    }
                    if !buf.as_slice().is_empty() {
                        let _ = bufwtr.print(&buf);
                    }
                }
                Err(err) => {
                    eprintln!("resharp: {}: {err}", entry.path().display());
                }
            }

            ignore::WalkState::Continue
        })
    });

    Ok((found_any.load(Ordering::Relaxed), had_errors.load(Ordering::Relaxed)))
}

fn walk_sorted(
    re: &resharp::Regex,
    args: &Args,
    paths: &[PathBuf],
    printer_opts: &PrinterOpts,
    color_choice: termcolor::ColorChoice,
    max_filesize: Option<u64>,
) -> anyhow::Result<(bool, bool)> {
    let walker = build_walker(args, paths, 1)?;
    let mut entries: Vec<PathBuf> = Vec::new();

    for entry in walker.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("resharp: {err}");
                continue;
            }
        };

        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        if let Some(max) = max_filesize {
            if entry.metadata().map_or(false, |m| m.len() > max) {
                continue;
            }
        }

        entries.push(entry.into_path());
    }

    entries.sort();

    let mut found_any = false;
    let mut had_errors = false;
    for path in &entries {
        let (found, had_error) = search::search_file(re, path, args, printer_opts, color_choice)?;
        if had_error {
            had_errors = true;
        }
        if found {
            found_any = true;
            if args.quiet {
                return Ok((true, had_errors));
            }
        }
    }

    Ok((found_any, had_errors))
}
