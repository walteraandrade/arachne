# Architecture

## Data Pipeline

```
git2::Repository
      │
      ▼
  read_repo()          ← src/git/repo.rs
      │                   walks commits, collects branches
      ▼
  RepoData             ← src/git/types.rs
  { commits, branches,    Vec<Commit>, Vec<BranchInfo>,
    branch_tips }         HashSet<Oid>
      │
      ▼
  Dag::from_repo_data  ← src/graph/dag.rs
      │                   builds adjacency, children map,
      │                   topo-sorts
      ▼
  branch_assign        ← src/graph/branch_assign.rs
      │                   assigns branch identity per commit
      ▼
  compute_layout()     ← src/graph/layout.rs
      │                   allocates lanes, reserves trunk,
      │                   builds edge segments
      ▼
  Vec<GraphRow>        ← src/graph/types.rs
      │                   one row per commit: oid, lane,
      │                   edges, summary, time, color
      ▼
  UI rendering         ← src/ui/graph_view.rs
                          renders rows with branch colors,
                          merge lines, selection highlight
```

## Module Structure

```
src/
├── main.rs              entry point, event loop, terminal setup
├── app.rs               App state, render orchestration, event dispatch
├── config.rs            TOML/env/CLI config via figment
├── error.rs             error types
├── event.rs             AppEvent enum (Key, FsChanged, GitHubUpdate, etc.)
├── git/
│   ├── repo.rs          open_repo, read_repo — git2 commit walking
│   └── types.rs         Commit, BranchInfo, RepoData, Oid
├── github/
│   ├── client.rs        GitHubClient — octocrab wrapper
│   ├── network.rs       fetch_network_detached — fork/branch fetching
│   └── types.rs         GitHub API response types
├── graph/
│   ├── dag.rs           Dag — adjacency list, topo sort, merge support
│   ├── branch_assign.rs branch identity propagation
│   ├── layout.rs        compute_layout — lane allocation, trunk reservation
│   └── types.rs         GraphRow, EdgeSegment
├── ui/
│   ├── graph_view.rs    GraphView widget — renders commit rows
│   ├── branch_panel.rs  BranchPanel — collapsible branch list
│   ├── detail_panel.rs  commit detail popup
│   ├── header_bar.rs    pane tabs + sync status
│   ├── help_panel.rs    keybinding reference overlay
│   ├── input.rs         key → Action mapping
│   ├── status_bar.rs    bottom bar — branch, sync, filter state
│   └── theme.rs         color constants
└── watcher/
    ├── fs.rs            notify-based filesystem watcher (debounced)
    └── poll.rs          periodic GitHub polling
```

## Event Architecture

```
crossterm EventStream (async)
      │
      ▼
  tokio::spawn → filters Key/Resize
      │
      ▼
  mpsc::unbounded_channel<AppEvent>
      │
      ├── FsChanged(pane_idx)      ← notify watcher (debounced)
      ├── GitHubUpdate(pane_idx)   ← periodic poll timer
      ├── GitHubResult { .. }      ← async fetch completion
      ├── Key(KeyEvent)            ← terminal input
      └── Resize                   ← terminal resize
      │
      ▼
  main loop:
    1. recv() first event (blocks)
    2. try_recv() drain remaining (batch)
    3. collapse FsChanged per pane → single rebuild_graph
    4. terminal.draw()
```

Key design choices:
- **current_thread tokio** — git2::Repository is !Send, avoids cross-thread shuffling
- **Event batching** — key repeat floods drained before redraw, keeps UI responsive
- **Debounced FS events** — multiple rapid file changes collapsed into one rebuild per pane

## Multi-Repo Time Sync

When multiple repos are open in split view, scrolling the active pane syncs others by timestamp. `pane_sync_info()` finds the closest commit in each non-active pane using absolute time difference, then centers the viewport around that commit.

## Trunk-Aware Layout

`compute_layout` reserves the leftmost lanes for configured trunk branches (e.g. development, staging, production). Feature branches are allocated to lanes right of the reserved block. This keeps trunk branches visually stable regardless of how many feature branches exist.
