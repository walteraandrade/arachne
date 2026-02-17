# Configuration

Arachne uses [figment](https://docs.rs/figment) for layered
configuration. Values are resolved in the following precedence order
(highest wins):

1. CLI flags (`--repo`)
2. `GITHUB_TOKEN` environment variable
3. `ARACHNE_`-prefixed environment variables
4. TOML config file
5. Built-in defaults

## Config file location

```
$XDG_CONFIG_HOME/arachne/config.toml
```

Falls back to `~/.config/arachne/config.toml` if `XDG_CONFIG_HOME`
isn't set. If the file doesn't exist, defaults are used silently.
Parse errors emit a warning and fall back to defaults.

## Reference

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `repo_path` | `string` | `"."` | Path to git repository (single-repo mode) |
| `github_token` | `string` | — | GitHub personal access token for network API |
| `poll_interval_secs` | `integer` | `60` | Seconds between GitHub network refreshes |
| `show_forks` | `boolean` | `true` | Include fork branches in the graph |
| `max_commits` | `integer` | `500` | Maximum commits to load per repository |
| `trunk_branches` | `string[]` | `["development", "staging", "production"]` | Branch names that get reserved leftmost lanes |
| `repos` | `RepoEntry[]` | `[]` | Multi-repo entries (overrides `repo_path`) |

### RepoEntry

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `path` | `string` | yes | Path to git repository |
| `name` | `string` | no | Display name (auto-detected from remote URL if omitted) |

## Single-repo mode

When `repos` is empty (the default), arachne opens a single pane
using `repo_path`. The `--repo` CLI flag overrides this value.

```toml
repo_path = "/home/user/projects/myapp"
show_forks = true
max_commits = 1000
trunk_branches = ["main", "staging", "production"]
```

## Multi-repo mode

Define multiple `[[repos]]` entries to open side-by-side panes.
When `repos` is non-empty, `repo_path` is ignored.

```toml
max_commits = 500
trunk_branches = ["main", "develop"]

[[repos]]
path = "~/Github/frontend"
name = "org/frontend"

[[repos]]
path = "~/Github/backend"
name = "org/backend"

[[repos]]
path = "~/Github/infra"
```

Pane widths are proportional to the square root of each repo's
commit count, giving larger repos more space without overwhelming
smaller ones.

## Environment variables

Set any config key with the `ARACHNE_` prefix:

```sh
export ARACHNE_MAX_COMMITS=1000
export ARACHNE_POLL_INTERVAL_SECS=120
export ARACHNE_SHOW_FORKS=false
```

The `GITHUB_TOKEN` variable is mapped directly to `github_token`
without requiring the prefix:

```sh
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

## CLI flags

| Flag | Description |
|------|-------------|
| `--repo`, `-r` | Path to git repository (overrides `repo_path`) |

## Trunk branches

Trunk branches get reserved lanes on the left side of the graph.
This keeps them visually stable regardless of how many feature
branches appear. The order in the array determines lane assignment
(first entry gets lane 0).

Branches are matched by name after stripping remote prefixes. For
example, `origin/production` matches the trunk entry `"production"`.

```toml
trunk_branches = ["main", "staging", "production"]
```

To disable trunk reservation, set an empty array:

```toml
trunk_branches = []
```

## GitHub token

Arachne uses the GitHub token to fetch fork and branch data from
the network API. Without a token, the graph shows only local data.

Create a [personal access token](https://github.com/settings/tokens)
with `repo` scope for private repositories, or `public_repo` for
public-only access.

The token is redacted in debug output.
