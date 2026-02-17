# Omarchy integration

Arachne integrates with [omarchy](https://github.com/nicholasgasior/omarchy)
as a first-class TUI application. Install files in the `install/`
directory follow omarchy surface conventions for desktop entries,
keybindings, status bar modules, and notifications.

## Surfaces

### Desktop entry

**File:** `install/applications/arachne.desktop`

The desktop entry uses `omarchy-launch-tui` as the launcher and
sets `StartupWMClass=org.omarchy.arachne` for window matching.

```sh
cp install/applications/arachne.desktop ~/.local/share/applications/
cp install/applications/icons/arachne.svg \
   ~/.local/share/icons/hicolor/scalable/apps/
gtk-update-icon-cache ~/.local/share/icons/hicolor/
```

### Hyprland keybinding

**File:** `install/hypr/bindings.conf`

Binds `Super+Shift+G` to launch or focus arachne via
`omarchy-launch-or-focus-tui`.

```sh
cp install/hypr/bindings.conf ~/.config/hypr/conf.d/arachne.conf
hyprctl reload
```

### Launcher script

**File:** `install/bin/arachne`

Wrapper script that locates the cargo-installed binary and execs
with passthrough args. Also runs `gtk-update-icon-cache` on launch.

```sh
cp install/bin/arachne ~/.local/bin/
```

### Waybar module

**Files:** `install/waybar/arachne.jsonc`, `install/waybar/arachne.css`

Static launcher button displaying a git icon. Click opens or
focuses arachne.

```sh
# merge arachne.jsonc into your waybar config modules
# include arachne.css in your waybar style
```

### Mako notification rule

**File:** `install/mako/arachne`

Click-to-focus rule for future notification features.

```sh
cp install/mako/arachne ~/.config/mako/criteria.d/
makoctl reload
```

## Signal allocation

| Signal | Purpose |
|--------|---------|
| `RTMIN+11` | Waybar arachne module refresh |

## Quick install

Copy all surfaces in one go:

```sh
cp install/applications/arachne.desktop \
   ~/.local/share/applications/
cp install/applications/icons/arachne.svg \
   ~/.local/share/icons/hicolor/scalable/apps/
cp install/bin/arachne ~/.local/bin/
cp install/hypr/bindings.conf \
   ~/.config/hypr/conf.d/arachne.conf
cp install/mako/arachne ~/.config/mako/criteria.d/
```

After copying, reload affected services:

```sh
gtk-update-icon-cache ~/.local/share/icons/hicolor/
hyprctl reload
makoctl reload
```
