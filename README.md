# re#

a grep that can search for multiple words at once, powered by [RE#](https://github.com/ieviev/resharp).

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

### Proximity search

`-P N` / `--near N` constrains matches so all terms appear within N lines of each other:

```sh
re# -P 5 -W unsafe -W unwrap src/    # "unsafe" and "unwrap" within 5 lines
re# -P 3 -W TODO -W FIXME .          # nearby TODOs and FIXMEs
re# -P 10 -W fn -W unsafe -t rust    # functions near unsafe blocks
```

`--near` composes with other flags:

```sh
re# -P 5 -W unsafe -W unwrap --not allow -t rust   # with --not
re# -P 5 -W unsafe -W unwrap -c src/               # count matches
re# -P 5 -W unsafe -W unwrap --json src/            # JSON output
```

### Paragraph search

`-p` searches paragraphs (blocks separated by blank lines) instead of lines:

```sh
re# -p error -p timeout               # paragraphs containing both words
re# -p error -p timeout -t rust       # only in rust files
re# -i -p error -p timeout            # case insensitive
```

### Scoped search

`--scope` controls the match boundary. the default is `line`.

| scope | flag | matches within |
|-------|------|----------------|
| line | (default) | single lines |
| paragraph | `-p` / `--scope paragraph` | blocks separated by blank lines |
| file | `--scope file` | entire files |
| custom | `--scope '<regex>'` | any regex constraint |

```sh
re# --scope file -W serde -W async -l src/  # files containing both words
re# --scope '(_*\n){0,3}' -W error -W warn src/  # custom: within 3 lines
```

### Pattern operators

`&` means AND - both sides must match.
`~` means NOT - exclude what matches.
`_` matches any single byte including newline (unlike `.` which stops at `\n`).

```sh
# hex strings that contain both a digit and a letter
re# '([0-9a-f]+)&(_*[0-9]_*)&(_*[a-f]_*)'

# identifiers 8-20 chars long containing "config"
re# '([a-zA-Z_]+)&(_{8,20})&(_*config_*)'

# lines NOT containing "debug"
re# '~(_*debug_*)' src/
```

try patterns interactively in the [web playground](https://ieviev.github.io/resharp-webapp/).

### How it works

every constraint is just a regex intersection. when you write:

```sh
re# -P 5 -W unsafe -W unwrap
```

re# builds the pattern:

```
(_*unsafe_*) & (_*unwrap_*) & ~((_*\n_*){6})
```

`-W` terms become intersections (`_*word_*`), `--near 5` rejects spans with 6+ newlines via complement (`~`), and scopes like `-p` or `--scope` add their own boundary constraint. everything composes through the same mechanism, so all features (highlighting, context, `--count`, `--json`, etc.) work uniformly.

### `_` wildcard

in RE#, `_` replaces `.` as the "match anything" character. the difference is `_` also matches newlines, which matters for paragraph search and multi-line patterns.

```sh
re# 'my_function'              # matches myXfunction, my.function, ...
re# 'my\_function'             # literal underscore
re# -R 'my_function'           # -R: standard regex mode (. and _ behave normally)
re# -F 'my_function'           # -F: fixed string, no regex at all
```

## Differences from `rg`

| `rg` | `re#` | why |
|------|-------|-----|
| `-a` / `--text` | `-uuu` | `-a` is taken by `--and` |
| `_` is literal | `_` is wildcard | use `-R` or `\_` for literal |
| pattern is standard regex | pattern has `&`, `~`, `_` operators | use `-R` for standard regex mode |

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
