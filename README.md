# re#

a grep that can search for multiple words at once and across paragraphs, powered by the [RE# regex engine](https://github.com/ieviev/resharp).

[install](#install) | [web playground](https://ieviev.github.io/resharp-webapp/)

> `re#` is a valid binary name on unix - `#` only starts a comment after whitespace.
> also included as `resharp` for compatibility.

## Quickstart

```sh
re# 'TODO' src/                       # find 'TODO' in src/
re# -i 'fixme' .                      # case insensitive
re# -w 'error' -t rust                # whole word, rust files only
echo 'hello world' | re# 'hello'      # stdin
```

### Multi-word search

`-W` finds lines containing all given words:

```sh
re# -W error -W timeout src/          # lines with both "error" AND "timeout"
re# -W error -W timeout -W retry .    # all three must appear
```

`--not` excludes lines matching a pattern:

```sh
re# -W error --not debug src/         # "error" without "debug"
re# -W error -W warn --not debug .    # "error" and "warn", but not "debug"
```

### Paragraph search

`-p` searches paragraphs (blocks separated by blank lines) instead of lines:

```sh
re# -p error -p timeout               # paragraphs containing both words
re# -p error -p timeout -t rust       # only in rust files
re# -i -p error -p timeout            # case insensitive
```

### Regex algebra

`&` (intersection) and `~` (complement) let you combine constraints in a single pattern.
`_` matches any character (like `.` but works across the algebra).

```sh
# hex strings that contain both a digit and a letter
re# '([0-9a-f]+)&(_*[0-9]_*)&(_*[a-f]_*)'

# lines with "error" AND "timeout" (same as -W, but inline)
re# '(_*error_*)&(_*timeout_*)'

# match identifiers 8-20 chars long containing "config"
re# '([a-zA-Z_]+)&(_{8,20})&(_*config_*)'

# complement: lines NOT containing "debug"
re# '~(_*debug_*)' src/
```

try patterns interactively in the [web playground](https://ieviev.github.io/resharp-webapp/).

## `_` = any byte in RE#

```sh
re# 'my_function'              # matches myXfunction, my.function, ...
re# 'my\_function'             # literal underscore
re# -R 'my_function'           # -R: compatibility mode
re# -F 'my_function'           # -F: fixed string, no regex at all
```

## Differences from `rg`

| `rg` | `re#` | why |
|------|-------|-----|
| `-a` / `--text` | `-uuu` | `-a` is taken by `--and` |
| `_` is literal | `_` is wildcard | use `-R` or `\_` for literal |
| pattern is standard regex | pattern has algebra | `&`, `~`, `_` are operators; use `-R` for compatibility mode |

## Exit codes

`0` match, `1` no match, `2` error

## Install

### Cargo

```sh
cargo install resharp-grep  # binary is named `resharp`
```

### Prebuilt binaries

download from [GitHub releases](https://github.com/ieviev/resharp-cli/releases).

### Nix

```sh
nix profile install github:ieviev/resharp-cli
```

or in a flake:

```nix
inputs.resharp.url = "github:ieviev/resharp-cli";
```

nix package includes both `resharp` and `re#`, plus shell completions.

## License

MIT
