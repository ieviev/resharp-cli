use std::io::{IsTerminal, Write};
use std::process::ExitCode;

mod args;
mod printer;
mod search;
mod walk;

fn main() -> ExitCode {
    match run() {
        Ok((_, true)) => ExitCode::from(2),
        Ok((true, _)) => ExitCode::from(0),
        Ok((false, _)) => ExitCode::from(1),
        Err(err) => {
            for cause in err.chain() {
                if let Some(ioerr) = cause.downcast_ref::<std::io::Error>() {
                    if ioerr.kind() == std::io::ErrorKind::BrokenPipe {
                        return ExitCode::from(0);
                    }
                }
            }
            let _ = writeln!(std::io::stderr(), "resharp: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> anyhow::Result<(bool, bool)> {
    let args = args::parse()?;

    if let Some(shell) = args.completions {
        let mut cmd = <args::Args as clap::CommandFactory>::command();
        let mut out = std::io::stdout();
        clap_complete::generate(shell, &mut cmd, "resharp", &mut out);
        clap_complete::generate(shell, &mut cmd, "re#", &mut out);
        return Ok((true, false));
    }

    if args.type_list {
        walk::print_type_list();
        return Ok((true, false));
    }

    let pattern = args.resolve_pattern()?;
    let engine_opts = resharp::EngineOptions {
        dfa_threshold: args.dfa_threshold,
        max_dfa_capacity: args.dfa_capacity,
        ..Default::default()
    };
    let re = resharp::Regex::with_options(&pattern, engine_opts)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let color_choice = args.color_choice();
    let printer_opts = printer::PrinterOpts::from_args(&args);

    if args.paths.is_empty() && !std::io::stdin().is_terminal() {
        let found = search::search_stdin(&re, &args, &printer_opts, color_choice)?;
        return Ok((found, false));
    }

    let paths: Vec<_> = if args.paths.is_empty() {
        vec![".".into()]
    } else {
        args.paths.clone()
    };

    for p in &paths {
        if !p.exists() {
            anyhow::bail!("{}: no such file or directory", p.display());
        }
    }

    walk::walk_and_search(&re, &pattern, &args, &paths, &printer_opts, color_choice)
}
