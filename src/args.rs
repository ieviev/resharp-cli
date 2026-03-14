use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "re#",
    about = "recursive search with boolean constraints, powered by resharp",
    version,
    before_help = "\x1b[1mExamples:\x1b[0m
  re# 'TODO' src/                       search like ripgrep
  re# -i 'error' .                      case insensitive
  re# error -a timeout src/              lines with both words
  re# error -N debug .                  \"error\" without \"debug\"
  re# -p error -p timeout -t rust       paragraphs with both words
  re# '(_*error_*)&~(_*debug_*)'        regex algebra
  re# serde -a async --scope file -l src/  files with both words
  re# --json 'TODO' src/               JSON output for agents
  re# -P 5 unsafe -a unwrap src/       proximity search",
    after_help = "resharp supports intersection (&), complement (~(...)), and universal wildcard (_).
see https://github.com/ieviev/resharp for the regex engine."
)]
pub struct Args {
    /// regex pattern (positional, mutually exclusive with -e/-f)
    #[arg(value_name = "PATTERN")]
    pub pattern: Option<String>,

    /// paths to search (files or directories)
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// regex pattern (repeatable, combined with |)
    #[arg(short = 'e', long = "regexp", value_name = "PATTERN")]
    pub regexp: Vec<String>,

    /// file containing patterns, one per line
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub pattern_file: Vec<PathBuf>,

    /// case insensitive search
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// case sensitive search (overrides -i)
    #[arg(short = 's', long = "case-sensitive")]
    pub case_sensitive: bool,

    /// smart case (insensitive unless pattern has uppercase)
    #[arg(short = 'S', long = "smart-case")]
    pub smart_case: bool,

    /// invert match (show non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// match only whole words
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// match only whole lines
    #[arg(short = 'x', long = "line-regexp")]
    pub line_regexp: bool,

    /// stop after NUM matches per file
    #[arg(short = 'm', long = "max-count", value_name = "NUM")]
    pub max_count: Option<usize>,

    /// suppress output, exit 0 if match found
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// print count of matches per file
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// print only filenames with matches
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// print only filenames without matches
    #[arg(long = "files-without-match")]
    pub files_without_match: bool,

    /// print only matched parts
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// show line numbers
    #[arg(short = 'n', long = "line-number")]
    pub line_number: bool,

    /// suppress line numbers
    #[arg(long = "no-line-number")]
    pub no_line_number: bool,

    /// show column number of first match
    #[arg(long = "column")]
    pub column: bool,

    /// show byte offset of match
    #[arg(short = 'b', long = "byte-offset")]
    pub byte_offset: bool,

    /// treat pattern as literal string
    /// with values: add literal string constraints (repeatable)
    /// e.g. -F lit1 -F lit2
    #[arg(short = 'F', long = "fixed-strings", aliases = ["fixed", "lit"], num_args = 0..=1, action = clap::ArgAction::Append, value_name = "STRING")]
    pub fixed_strings: Option<Vec<String>>,

    /// raw regex mode (standard regex, _ is literal, no resharp algebra)
    #[arg(short = 'R', long = "raw")]
    pub raw: bool,

    /// require PATTERN to also appear in match (repeatable)
    #[arg(short = 'a', long = "and", visible_short_alias = 'W', visible_alias = "with", alias = "add", value_name = "PATTERN")]
    pub and: Vec<String>,

    /// exclude matches containing PATTERN (repeatable)
    #[arg(short = 'N', long = "not", value_name = "PATTERN")]
    pub not: Vec<String>,

    /// replace matches with TEXT in output
    #[arg(long = "replace", value_name = "TEXT")]
    pub replace: Option<String>,

    /// lines of context after match
    #[arg(short = 'A', long = "after-context", value_name = "NUM")]
    pub after_context: Option<usize>,

    /// lines of context before match
    #[arg(short = 'B', long = "before-context", value_name = "NUM")]
    pub before_context: Option<usize>,

    /// lines of context before and after match
    #[arg(short = 'C', long = "context", value_name = "NUM")]
    pub context: Option<usize>,

    /// color output: auto, always, never
    #[arg(long = "color", value_name = "WHEN", default_value = "auto")]
    pub color: String,

    /// group results by file (default for terminal)
    #[arg(long = "heading")]
    pub heading: bool,

    /// don't group results by file
    #[arg(long = "no-heading")]
    pub no_heading: bool,

    /// include/exclude glob (repeatable, prefix with ! to exclude)
    #[arg(short = 'g', long = "glob", value_name = "GLOB")]
    pub glob: Vec<String>,

    /// case-insensitive glob
    #[arg(long = "iglob", value_name = "GLOB")]
    pub iglob: Vec<String>,

    /// search only files matching TYPE
    #[arg(short = 't', long = "type", value_name = "TYPE")]
    pub file_type: Vec<String>,

    /// exclude files matching TYPE
    #[arg(short = 'T', long = "type-not", value_name = "TYPE")]
    pub type_not: Vec<String>,

    /// list available file types
    #[arg(long = "type-list")]
    pub type_list: bool,

    /// search hidden files and directories
    #[arg(long = "hidden")]
    pub hidden: bool,

    /// don't respect ignore files
    #[arg(long = "no-ignore")]
    pub no_ignore: bool,

    /// don't respect .gitignore
    #[arg(long = "no-ignore-vcs")]
    pub no_ignore_vcs: bool,

    /// reduce filtering (-u: hidden, -uu: +no-ignore, -uuu: +binary)
    #[arg(short = 'u', long = "unrestricted", action = clap::ArgAction::Count)]
    pub unrestricted: u8,

    /// follow symbolic links
    #[arg(short = 'L', long = "follow")]
    pub follow: bool,

    /// number of search threads (0 = auto)
    #[arg(short = 'j', long = "threads", value_name = "NUM")]
    pub threads: Option<usize>,

    /// max directory depth
    #[arg(long = "max-depth", value_name = "NUM")]
    pub max_depth: Option<usize>,

    /// skip files larger than NUM bytes (supports K, M, G suffixes)
    #[arg(long = "max-filesize", value_name = "NUM+SUFFIX?")]
    pub max_filesize: Option<String>,

    /// sort results by path
    #[arg(long = "sort", value_name = "CRITERION")]
    pub sort: Option<String>,

    /// allow matches to span multiple lines
    #[arg(long = "multiline")]
    pub multiline: bool,

    /// constrain matches to within paragraphs
    /// with words: find paragraphs containing all words (intersection)
    /// e.g. -p word1 -p word2 -p word3
    #[arg(short = 'p', long = "paragraphs", alias = "§", num_args = 0..=1, action = clap::ArgAction::Append, value_name = "WORD")]
    pub paragraphs: Option<Vec<String>>,

    /// use memory-mapped I/O (auto: mmap files >= 1MB)
    #[arg(long = "mmap")]
    pub mmap: bool,

    /// disable memory-mapped I/O
    #[arg(long = "no-mmap")]
    pub no_mmap: bool,

    /// max DFA state capacity (default: 65535)
    #[arg(long = "dfa-capacity", value_name = "NUM", default_value = "65535")]
    pub dfa_capacity: usize,

    /// DFA precompilation threshold (default: 0)
    #[arg(long = "dfa-threshold", value_name = "NUM", default_value = "0")]
    pub dfa_threshold: usize,

    /// generate shell completions (bash, zsh, fish, elvish, powershell)
    #[arg(long = "completions", value_name = "SHELL", hide = true)]
    pub completions: Option<clap_complete::Shell>,

    /// scope for intersection (line, paragraph, file, or a boundary regex)
    #[arg(long = "scope", value_name = "SCOPE")]
    pub scope: Option<String>,

    /// find patterns within N lines of each other (use with --and)
    #[arg(short = 'P', long = "near", value_name = "NUM")]
    pub near: Option<usize>,

    /// stop after NUM total matches across all files
    #[arg(long = "max-total", value_name = "NUM")]
    pub max_total: Option<usize>,

    /// deduplicate matched strings (useful with -o)
    #[arg(long = "unique")]
    pub unique: bool,

    /// output results as JSON (one object per match line)
    #[arg(long = "json")]
    pub json: bool,

    /// delimiters for block scope (e.g., "{}")
    #[arg(long = "delimiters", value_name = "PAIR")]
    pub delimiters: Option<String>,

    /// show the enclosing function or block header for each match
    #[arg(long = "show-scope")]
    pub show_scope: bool,
}

impl Args {
    pub fn is_fixed_strings(&self) -> bool {
        // bare -F (no values): global fixed-strings mode
        // -F with values: only the -F terms are literal, not a global flag
        matches!(&self.fixed_strings, Some(v) if v.is_empty())
    }

    fn fixed_words(&self) -> &[String] {
        match &self.fixed_strings {
            Some(words) => words.as_slice(),
            None => &[],
        }
    }

    pub fn is_paragraph_mode(&self) -> bool {
        self.paragraphs.is_some()
    }

    pub fn effective_scope(&self) -> &str {
        if let Some(ref scope) = self.scope {
            scope.as_str()
        } else if self.is_paragraph_mode() {
            "paragraph"
        } else if self.multiline || self.near.is_some() {
            "multiline"
        } else {
            "line"
        }
    }

    /// resolve the final regex pattern from positional, -e, -f, -W, -F, and -p flags
    pub fn resolve_pattern(&self) -> anyhow::Result<String> {
        let para: Vec<&str> = match &self.paragraphs {
            Some(w) => w.iter().map(|s| s.as_str()).collect(),
            None => vec![],
        };
        // -a is a modifier when a base pattern exists, standalone otherwise
        let has_base = self.pattern.is_some()
            || !self.regexp.is_empty()
            || !self.pattern_file.is_empty();
        let words: Vec<&str> = if has_base {
            para
        } else {
            para.into_iter().chain(self.and.iter().map(|s| s.as_str())).collect()
        };
        let fixed = self.fixed_words();
        let total_terms = words.len() + fixed.len();

        if total_terms > 0 {
            if !self.regexp.is_empty() || !self.pattern_file.is_empty() {
                anyhow::bail!("-e/-f cannot be combined with -W/-p/-F word patterns");
            }
            if self.near.is_some() && total_terms < 2 {
                anyhow::bail!("--near requires at least 2 terms (use -W, --and, or & in pattern)");
            }
            return Ok(self.build_words_pattern(&words));
        }

        let mut patterns = Vec::new();

        // -e patterns
        patterns.extend(self.regexp.iter().cloned());

        // -f patterns
        for path in &self.pattern_file {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("failed to read pattern file {}: {e}", path.display()))?;
            for line in contents.lines() {
                if !line.is_empty() {
                    patterns.push(line.to_string());
                }
            }
        }

        // positional pattern (only if no -e/-f)
        if patterns.is_empty() {
            match &self.pattern {
                Some(p) => patterns.push(p.clone()),
                None => anyhow::bail!("no pattern provided"),
            }
        }

        // apply transformations
        let patterns: Vec<String> = patterns
            .into_iter()
            .map(|p| self.wrap_pattern(p))
            .collect();

        // combine with alternation
        let mut combined = if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            patterns
                .into_iter()
                .map(|p| format!("({p})"))
                .collect::<Vec<_>>()
                .join("|")
        };

        // apply --and / --not intersection/complement
        combined = self.apply_and_not(combined);

        // --near requires at least 2 terms
        if self.near.is_some() && self.and.is_empty() {
            anyhow::bail!("--near requires at least 2 terms (use -W, --and, or & in pattern)");
        }

        // apply scope boundary
        combined = self.apply_scope_boundary(combined);

        Ok(combined)
    }

    /// build pattern from -W/-p/-F words: intersect all with _*word_*, apply scope boundary
    fn build_words_pattern(&self, words: &[&str]) -> String {
        let mut terms: Vec<String> = words
            .iter()
            .map(|w| {
                let wrapped = self.wrap_pattern(w.to_string());
                format!("(_*{wrapped}_*)")
            })
            .collect();

        // -F adds literal string constraints (pre-escaped)
        for f in self.fixed_words() {
            let escaped = resharp::escape(f);
            terms.push(format!("(_*{escaped}_*)"));
        }

        let mut combined = terms.join("&");

        // --not: complement within scope
        if !self.not.is_empty() {
            if self.effective_scope() == "line" {
                for n in &self.not {
                    let term = self.wrap_pattern(n.clone());
                    combined = format!("{combined}&~(.*{term}.*)");
                }
                combined = format!("^({combined})$");
            } else {
                for n in &self.not {
                    let term = self.wrap_pattern(n.clone());
                    combined = format!("{combined}&~(_*{term}_*)");
                }
            }
        }

        self.apply_scope_boundary(combined)
    }

    fn apply_and_not(&self, mut combined: String) -> String {
        let has_and = !self.and.is_empty();
        let has_not = !self.not.is_empty();
        if !has_and && !has_not {
            return combined;
        }

        if self.effective_scope() == "line" {
            combined = format!("(.*{combined}.*)");
            for a in &self.and {
                let term = self.wrap_pattern(a.clone());
                combined = format!("{combined}&(.*{term}.*)");
            }
            for n in &self.not {
                let term = self.wrap_pattern(n.clone());
                combined = format!("{combined}&~(.*{term}.*)");
            }
            combined = format!("^({combined})$");
        } else {
            combined = format!("(_*{combined}_*)");
            for a in &self.and {
                let term = self.wrap_pattern(a.clone());
                combined = format!("{combined}&(_*{term}_*)");
            }
            for n in &self.not {
                let term = self.wrap_pattern(n.clone());
                combined = format!("{combined}&~(_*{term}_*)");
            }
        }

        combined
    }

    fn apply_scope_boundary(&self, combined: String) -> String {
        let scoped = match self.effective_scope() {
            "line" => format!("({combined})&(.*)"),
            "paragraph" => format!("({combined})&((?<=\\A|\n\n)(_*&~(_*\n\n_*)&~(\n_*|_*\n))(?=\n\n|\n\\z|\\z))"),
            "file" | "multiline" => combined,
            custom => format!("({combined})&~(_*{custom}_*)"),
        };
        self.apply_near(scoped)
    }

    fn apply_near(&self, combined: String) -> String {
        match self.near {
            Some(n) => format!("({combined})&~((_*\n_*){{{}}})", n + 1),
            None => combined,
        }
    }

    fn wrap_pattern(&self, mut pattern: String) -> String {
        if self.is_fixed_strings() {
            pattern = resharp::escape(&pattern);
        } else if self.raw {
            pattern = Self::escape_resharp(&pattern);
        }

        if self.word_regexp {
            pattern = format!(r"\b({pattern})\b");
        }

        if self.line_regexp {
            pattern = format!("^({pattern})$");
        }

        if self.should_ignore_case(&pattern) {
            pattern = format!("(?i){pattern}");
        }

        pattern
    }

    /// escape resharp-specific metacharacters (_ & ~) so the pattern
    /// is treated as standard regex only
    fn escape_resharp(pattern: &str) -> String {
        let mut out = String::with_capacity(pattern.len());
        let mut chars = pattern.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                out.push('\\');
                if chars.peek().is_some() {
                    out.push(chars.next().unwrap());
                }
            } else if c == '_' || c == '&' || c == '~' {
                out.push('\\');
                out.push(c);
            } else {
                out.push(c);
            }
        }
        out
    }

    fn should_ignore_case(&self, pattern: &str) -> bool {
        if self.case_sensitive {
            return false;
        }
        if self.ignore_case {
            return true;
        }
        if self.smart_case {
            return !pattern.chars().any(|c| c.is_uppercase());
        }
        false
    }

    pub fn color_choice(&self) -> termcolor::ColorChoice {
        if self.json {
            return termcolor::ColorChoice::Never;
        }
        match self.color.as_str() {
            "always" => termcolor::ColorChoice::Always,
            "never" => termcolor::ColorChoice::Never,
            _ => {
                if std::io::stdout().is_terminal() {
                    termcolor::ColorChoice::Auto
                } else {
                    termcolor::ColorChoice::Never
                }
            }
        }
    }

    pub fn effective_hidden(&self) -> bool {
        self.hidden || self.unrestricted >= 1
    }

    pub fn effective_no_ignore(&self) -> bool {
        self.no_ignore || self.unrestricted >= 2
    }

    pub fn search_binary(&self) -> bool {
        self.unrestricted >= 3
    }

    pub fn show_line_number(&self, multi_file: bool) -> bool {
        if self.no_line_number {
            return false;
        }
        self.line_number || multi_file
    }

    pub fn show_heading(&self) -> bool {
        if self.json {
            return false;
        }
        if self.no_heading {
            return false;
        }
        self.heading || std::io::stdout().is_terminal()
    }

    pub fn after_ctx(&self) -> usize {
        self.after_context.or(self.context).unwrap_or(0)
    }

    pub fn before_ctx(&self) -> usize {
        self.before_context.or(self.context).unwrap_or(0)
    }

    /// whether to use mmap for a file of the given size
    pub fn use_mmap(&self, file_size: u64) -> bool {
        if self.no_mmap {
            return false;
        }
        if self.mmap {
            return true;
        }
        file_size >= 1024 * 1024
    }

    pub fn parse_max_filesize(&self) -> anyhow::Result<Option<u64>> {
        let s = match self.max_filesize.as_ref() {
            Some(s) => s,
            None => return Ok(None),
        };
        let s = s.trim();
        let (num_str, multiplier) = if s.ends_with('K') || s.ends_with('k') {
            (&s[..s.len() - 1], 1024u64)
        } else if s.ends_with('M') || s.ends_with('m') {
            (&s[..s.len() - 1], 1024 * 1024)
        } else if s.ends_with('G') || s.ends_with('g') {
            (&s[..s.len() - 1], 1024 * 1024 * 1024)
        } else {
            (s.as_ref(), 1u64)
        };
        let n = num_str.parse::<u64>()
            .map_err(|_| anyhow::anyhow!("invalid --max-filesize value: {s}"))?;
        Ok(Some(n * multiplier))
    }
}

pub fn parse() -> anyhow::Result<Args> {
    let mut args = Args::parse();

    // when -e/-f/-F is used or -p has words, positional PATTERN is a PATH
    // -a is a modifier, not a pattern source, so it doesn't cause reinterpretation
    let has_words = args.paragraphs.as_ref().map_or(false, |v| !v.is_empty())
        || args.fixed_strings.as_ref().map_or(false, |v| !v.is_empty());
    if (!args.regexp.is_empty() || !args.pattern_file.is_empty() || has_words)
        && args.pattern.is_some()
    {
        let pat = args.pattern.take().unwrap();
        args.paths.insert(0, PathBuf::from(pat));
    }

    // --scope paragraph is equivalent to -p (without words)
    if args.scope.as_deref() == Some("paragraph") && args.paragraphs.is_none() {
        args.paragraphs = Some(Vec::new());
    }

    Ok(args)
}
