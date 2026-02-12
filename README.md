# arachne

TUI git network graph viewer — replaces GitHub's Network tab, right in your terminal.

## Features

- Git commit DAG visualization with branch lanes
- GitHub integration: fetches fork/branch data from the network
- Live filesystem watching — graph updates on new commits
- Periodic GitHub polling for remote changes
- Branch panel, commit detail panel, and status bar
- Configurable via TOML, env vars, or CLI flags

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

```toml
repo_path = "."
show_forks = true
max_commits = 500
poll_interval_secs = 60
```

GitHub token via env:

```sh
export GITHUB_TOKEN=ghp_...
```

All config keys can also be set with `ARACHNE_` prefix env vars.

## License

MIT
