# Architecture

Arachne transforms raw git data into a visual DAG through a
multi-stage pipeline. Each stage is isolated in its own module,
making the flow easy to follow and test independently.

## Data pipeline

```
git2::Repository
      |
      v
  read_repo()          <- src/git/repo.rs
      |                   walks commits, collects branches/tags
      v
  RepoData             <- src/git/types.rs
  { commits, branches,    Vec<CommitInfo>, Vec<BranchInfo>,
    tags, branch_tips }   HashSet<Oid>
      |
      v
  Dag::from_repo_data  <- src/graph/dag.rs
      |                   builds adjacency + children map,
      |                   Kahn's topo sort w/ time tiebreak
      v
  branch_assign        <- src/graph/branch_assign.rs
      |                   assigns branch identity per commit
      |                   (trunk first, then features)
      v
  compute_layout()     <- src/graph/layout.rs
      |                   allocates lanes, reserves trunk columns,
      |                   builds merge/branch edge segments
      v
  Vec<GraphRow>        <- src/graph/types.rs
      |                   one row per commit: cells, oid, message,
      |                   author, time, source, branch/tag names
      v
  UI rendering         <- src/ui/graph_view.rs
                          renders rows with branch colors,
                          merge lines, selection highlight
```

## Module structure

```
src/
+-- main.rs              entry point, event loop, terminal setup
+-- app.rs               App state, render orchestration, event dispatch
+-- config.rs            TOML/env/CLI config via figment
+-- error.rs             ArachneError enum + Result alias
+-- event.rs             AppEvent enum (Key, FsChanged, GitHubUpdate, etc.)
+-- test_utils.rs        test helpers (make_oid, make_commit, make_repo_data)
+-- git/
|   +-- repo.rs          open_repo, read_repo -- git2 commit walking
|   +-- types.rs         CommitInfo, BranchInfo, TagInfo, RepoData, Oid
+-- github/
|   +-- client.rs        GitHubClient -- octocrab wrapper, paginated fetches
|   +-- network.rs       fetch_network_detached -- fork/branch/commit collection
|   +-- types.rs         ForkInfo
+-- graph/
|   +-- dag.rs           Dag -- adjacency list, Kahn's topo sort, merge support
|   +-- branch_assign.rs branch identity propagation (trunk-first)
|   +-- filter.rs        author filter w/ parent-edge rewriting
|   +-- layout.rs        compute_layout -- lane allocation, trunk reservation
|   +-- types.rs         GraphRow, CellSymbol, Cell, LayoutState
+-- ui/
|   +-- graph_view.rs    GraphView widget -- renders commit rows
|   +-- branch_panel.rs  BranchPanel -- collapsible branch list w/ sections
|   +-- detail_panel.rs  commit detail popup
|   +-- header_bar.rs    pane tabs + filter display + sync status
|   +-- help_panel.rs    keybinding reference overlay
|   +-- input.rs         key -> Action mapping, FilterMode
|   +-- status_bar.rs    bottom bar -- branch, sync, filter state, hints
|   +-- theme.rs         color constants, branch_color_by_identity
+-- watcher/
    +-- fs.rs            notify-based filesystem watcher (300ms debounce)
    +-- poll.rs          periodic GitHub polling timer
```

## Event architecture

```
crossterm EventStream (async)
      |
      v
  tokio::spawn -> filters Key/Resize
      |
      v
  mpsc::unbounded_channel<AppEvent>
      |
      +-- FsChanged(pane_idx)      <- notify watcher (debounced)
      +-- GitHubUpdate(pane_idx)   <- periodic poll timer
      +-- GitHubResult { .. }      <- async fetch completion
      +-- Key(KeyEvent)            <- terminal input
      +-- Resize                   <- terminal resize
      |
      v
  main loop:
    1. recv() first event (blocks)
    2. try_recv() drain remaining (batch)
    3. collapse FsChanged per pane -> single rebuild_graph
    4. terminal.draw()
```

## Key design decisions

**`current_thread` tokio runtime.** `git2::Repository` is `!Send`,
so a multi-threaded runtime would require wrapping it in unsafe
constructs or spawning blocking threads. The single-threaded runtime
avoids this entirely while still supporting async I/O for GitHub
fetches and event streams.

**Event batching.** Terminal key repeat can flood the channel with
dozens of events between frames. The main loop drains all pending
events with `try_recv()` after the first blocking `recv()`, then
processes them in batch before a single redraw. This keeps the UI
responsive under rapid input.

**FS watcher debounce.** Git operations like `commit` or `rebase`
can trigger multiple rapid filesystem events (HEAD update, ref
update, packed-refs). The watcher sleeps 300ms after the first
event, drains any remaining events, then emits a single
`FsChanged` per pane. This avoids redundant graph rebuilds.

**Kahn's algorithm with time tiebreak.** The DAG topological sort
uses Kahn's algorithm with a `BinaryHeap<(DateTime, Oid)>` for
tiebreaking. When multiple commits have zero in-degree, the newest
one is processed first. This produces a natural-feeling ordering
that roughly matches `git log` output.

## GitHub network merge

When GitHub data arrives, `Dag::merge_remote()` integrates remote
commits into the existing local DAG:

1. New commits (by Oid) are inserted into the adjacency map
2. Existing commits skip insertion (dedup by Oid)
3. Children maps are rewired for new parent relationships
4. The full DAG is re-sorted via Kahn's algorithm

This merge happens asynchronously — the fetch runs in a spawned
task and delivers results via `GitHubResult` events. The main loop
processes the result and triggers a layout recompute.

## Multi-repo time sync

When multiple repos are open in split view, scrolling the active
pane syncs the others by timestamp:

1. `pane_sync_info()` reads the selected commit's timestamp from
   the active pane
2. Each non-active pane runs `find_closest_by_time_binary()` over
   a pre-sorted `time_sorted_indices` vector
3. The binary search (`partition_point`) finds the closest commit
   by absolute time difference in O(log n)
4. The viewport centers around the matched commit

## Trunk-aware layout

`compute_layout()` reserves the leftmost N lanes for configured
trunk branches (for example, `development`, `staging`,
`production`). Feature branches are allocated to lanes right of
the reserved block via `allocate_column_nonreserved()`.

This keeps trunk branches visually anchored in fixed positions
regardless of how many feature branches exist, making the graph
easier to scan for mainline history.

## Author filter with edge rewriting

`filter_by_author()` doesn't just hide non-matching commits — it
rewrites parent edges so the filtered graph remains connected.
When a non-matching commit sits between two matching ones, a BFS
skip-cache finds the nearest matching ancestors and creates direct
edges. Branches with no matching commits are pruned entirely.
