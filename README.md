# arachne

Named after Arachne, the mortal weaver of Greek myth — the network
graph is the web.

Arachne is a TUI git network graph viewer that replaces GitHub's
Network tab, right in your terminal. It renders a full DAG
visualization with branch lanes, merge edges, and multi-repo split
views — all driven by live filesystem watching and optional GitHub
network data.

## Features

- **DAG visualization** — commit graph with branch lanes, merge
  edges, and trunk-aware column reservation
- **Multi-repo split view** — side-by-side panes with time-synced
  scrolling across repositories
- **GitHub network** — fetches fork and branch data from GitHub's
  API, merged into the local graph
- **Live watching** — filesystem watcher triggers graph rebuilds on
  new commits, rebases, and ref changes
- **Trunk-aware layout** — reserved lanes for trunk branches keep
  them visually stable regardless of feature branch count
- **Branch panel** — collapsible sections for local, remote, fork,
  and tag refs with two-tone prefix coloring
- **Filtering** — branch name filter (`/`) and author filter (`a`)
  with real-time graph updates
- **Periodic polling** — GitHub data refreshes on a configurable
  interval with rate-limit awareness

## Prerequisites

- **Rust toolchain** — 1.70+ (2021 edition)
- **libgit2** — typically bundled by the `git2` crate, but some
  distros require `libgit2-dev` or `libgit2-devel`
- **GitHub token** (optional) — required for fork/network data.
  Create a [personal access token](https://github.com/settings/tokens)
  with `repo` scope

## Install

```sh
cargo install --path .
```

Or build and run directly:

```sh
cargo build --release
./target/release/arachne
```

## Usage

```sh
# current directory
arachne

# specific repo
arachne --repo /path/to/repo
```

## Configuration

Config file location: `~/.config/arachne/config.toml`

### Single repo

```toml
repo_path = "."
show_forks = true
max_commits = 500
poll_interval_secs = 60
trunk_branches = ["development", "staging", "production"]
```

### Multi-repo

```toml
max_commits = 500
trunk_branches = ["main", "develop"]

[[repos]]
path = "~/Github/frontend"
name = "org/frontend"

[[repos]]
path = "~/Github/backend"
name = "org/backend"
```

### Environment variables

```sh
export GITHUB_TOKEN=ghp_...
```

All config keys can also be set with the `ARACHNE_` prefix (for
example, `ARACHNE_MAX_COMMITS=1000`).

For the full configuration reference, see
[docs/configuration.md](docs/configuration.md).

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `h` / `←` | Focus branch panel |
| `l` / `→` | Focus graph |
| `H` | Scroll graph left |
| `L` | Scroll graph right |
| `Tab` | Next pane |
| `Shift+Tab` | Previous pane |
| `Enter` | Toggle detail / expand section |
| `f` | Toggle fork branches |
| `/` | Branch filter |
| `a` | Author filter |
| `r` | Refresh |
| `?` | Help |
| `Esc` | Close popup / cancel filter |
| `q` | Quit |

## Documentation

- [Architecture](docs/architecture.md) — data pipeline, module
  structure, event architecture, design decisions
- [Configuration](docs/configuration.md) — full config reference
  with all keys, types, defaults, and precedence
- [Omarchy integration](docs/omarchy-integration.md) — desktop entry,
  Hyprland keybinding, waybar module, mako notification setup

## License

[MIT](LICENSE)
