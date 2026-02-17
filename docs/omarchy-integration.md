# Omarchy Integration

Install files live in `install/` and follow the omarchy surface conventions.

## Surfaces

### Desktop Entry

**File:** `install/applications/arachne.desktop`

```sh
cp install/applications/arachne.desktop ~/.local/share/applications/
cp install/applications/icons/arachne.svg ~/.local/share/icons/hicolor/scalable/apps/
gtk-update-icon-cache ~/.local/share/icons/hicolor/
```

### Hyprland Keybinding

**File:** `install/hypr/bindings.conf`

Binds `Super+Shift+G` to launch/focus arachne.

```sh
cp install/hypr/bindings.conf ~/.config/hypr/conf.d/arachne.conf
hyprctl reload
```

### Launcher Script

**File:** `install/bin/arachne`

Wrapper that finds the cargo-installed binary and execs with passthrough args.

```sh
cp install/bin/arachne ~/.local/bin/
```

### Waybar Module

**Files:** `install/waybar/arachne.jsonc`, `install/waybar/arachne.css`

Static launcher button with `󰊢` icon. Click opens/focuses arachne.

```sh
# Merge arachne.jsonc into your waybar config modules
# Include arachne.css in your waybar style
```

### Mako Notification Rule

**File:** `install/mako/arachne`

Click-to-focus rule for future notification features.

```sh
cp install/mako/arachne ~/.config/mako/criteria.d/
makoctl reload
```

## Signal Allocation

| Signal | Purpose |
|--------|---------|
| RTMIN+11 | Waybar arachne module refresh |

## Quick Install (All Surfaces)

```sh
cp install/applications/arachne.desktop ~/.local/share/applications/
cp install/applications/icons/arachne.svg ~/.local/share/icons/hicolor/scalable/apps/
cp install/bin/arachne ~/.local/bin/
cp install/hypr/bindings.conf ~/.config/hypr/conf.d/arachne.conf
cp install/mako/arachne ~/.config/mako/criteria.d/
```
