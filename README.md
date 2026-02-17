# arachne

Named after Arachne, the mortal weaver of Greek myth — the network graph is the web.

TUI git network graph viewer that replaces GitHub's Network tab, right in your terminal.

## Features

- **DAG visualization** — commit graph with branch lanes and merge edges
- **Multi-repo split view** — side-by-side panes, time-synced scrolling
- **GitHub network** — fetches fork/branch data from GitHub's network API
- **Live watching** — filesystem watcher updates graph on new commits
- **Trunk-aware layout** — reserved lanes for trunk branches (development, staging, production)
- **Branch panel** — collapsible sections for local, remote, and fork branches
- **Filtering** — branch name filter (`/`) and author filter (`a`)
- **Periodic polling** — GitHub data refreshes on a configurable interval

## Install

```sh
cargo install --path .
```

## Usage

```sh
# current directory
arachne

# specific repo
arachne --repo /path/to/repo
```

## Configuration

Config file: `~/.config/arachne/config.toml`

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

### Environment

```sh
export GITHUB_TOKEN=ghp_...
```

All config keys can also be set with `ARACHNE_` prefix env vars.

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

## Architecture

See [docs/architecture.md](docs/architecture.md) for the data pipeline, module structure, and event architecture.

## Omarchy Integration

See [docs/omarchy-integration.md](docs/omarchy-integration.md) for desktop entry, Hyprland keybinding, waybar module, and mako notification setup.

## License

MIT
