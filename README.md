# re#

a recursive search tool like ripgrep, but with boolean constraints - find lines (or paragraphs, or files) matching all of your terms at once. powered by [RE#](https://github.com/ieviev/resharp).

[install](#install) | [web playground](https://ieviev.github.io/resharp-webapp/)

> `re#` is a valid binary name on unix - `#` only starts a comment after whitespace.
> also included as `resharp` for compatibility.

## basic usage

```sh
re# 'TODO' src/                       # search like ripgrep
re# -a error -a timeout src/          # lines with both "error" AND "timeout"
re# -a error --not debug src/         # "error" but not "debug"
re# -F 'std::io' -F 'Error' src/     # same, but with literal strings
```

## adding constraints

`-a` (think `add`, `and`) requires each term to appear within the match scope (default: line).

| flag | effect |
|------|--------|
| `-a` / `--and` | must contain pattern |
| `-F` / `--fixed-strings` | must contain literal string (no regex) |
| `-N` / `--not` | must not contain pattern |

`-W` / `--with` is an alias for `-a`.

## controlling scope

by default, all constraints must be satisfied within a single line. `--scope` changes this boundary:

| scope | how to use | constraints must match within |
|-------|-----------|-------------------------------|
| line | (default) | a single line |
| paragraph | `-p` or `--scope paragraph` | text blocks separated by blank lines |
| file | `--scope file` | anywhere in the same file |
| custom | `--scope '<pattern>'` | match must not cross the pattern |

```sh
re# -p error -p timeout               # paragraphs containing both words
re# --scope file -a serde -a async -l src/  # list files containing both words
re# --scope='---' -a error -a warn .  # within the same --- delimited block
```

`-p word` is shorthand for `--scope paragraph -a word`.

### proximity search

`-P N` / `--near N` constrains all terms to appear within N lines of each other:

```sh
re# -P 5 -a unsafe -a unwrap src/    # "unsafe" and "unwrap" within 5 lines
re# -P 3 -a TODO -a FIXME .          # nearby TODOs and FIXMEs
```

## the RE# pattern language

beyond flag-based constraints, you can write patterns directly using operators that standard regex doesn't have:

| operator | meaning | example |
|----------|---------|---------|
| `&` | intersection - both sides must match | `(foo)&(bar)` |
| `~` | complement - exclude what follows | `~(_*debug_*)` |
| `_` | wildcard - like `.` but also matches newlines | `_*error_*` |

`_` is what makes multi-line and paragraph searches work. use `\_` for a literal underscore, `-R` for standard regex mode, or `-F` for fixed strings.

```sh
re# '([0-9a-f]+)&(_*[0-9]_*)&(_*[a-f]_*)'   # hex with both a digit and a letter
re# '([a-zA-Z_]+)&(_{8,20})&(_*config_*)'    # 8-20 char identifiers with "config"
re# '^(~(_*debug_*))$' src/                     # lines NOT containing "debug"
```

try patterns interactively in the [web playground](https://ieviev.github.io/resharp-webapp/).

## differences from ripgrep

most ripgrep flags work the same. the differences:

| ripgrep | re# | reason |
|---------|-----|--------|
| `-a` / `--text` | `-uuu` | `-a` is `--and` in re# |
| `_` is literal | `_` is wildcard | use `-R` or `\_` for literal |
| standard regex only | `&`, `~`, `_` operators | use `-R` for standard regex mode |

## exit codes

`0` match found, `1` no match, `2` error

## install

### cargo

```sh
cargo install resharp-grep  # installs binary named `resharp`
```

### prebuilt binaries

download from [GitHub releases](https://github.com/ieviev/resharp-cli/releases).

### nix

```sh
nix profile install github:ieviev/resharp-cli
```

or in a flake:

```nix
inputs.resharp.url = "github:ieviev/resharp-cli";
```

nix package includes both `resharp` and `re#`, plus shell completions.

## how it works

every flag-based feature compiles down to a [RE#](https://github.com/ieviev/resharp) pattern. for example:

```sh
re# -P 5 -a unsafe -a unwrap
```

builds:

```
(_*unsafe_*) & (_*unwrap_*) & ~((_*\n_*){6})
```

`-a` terms become intersections (`_*word_*`), `--near 5` rejects spans with 6+ newlines via complement (`~`), and scopes add their own boundary constraint. because everything compiles to the same representation, all output modes (highlighting, context, `--count`, `--json`, etc.) work uniformly.

see the [RE# engine](https://github.com/ieviev/resharp) for more on the regex algebra.

## license

MIT
