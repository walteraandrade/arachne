use crate::graph::pixel_renderer::RenderParams;

#[derive(Debug, Clone)]
pub enum GraphicsCapability {
    Kitty { cell_width: u16, cell_height: u16 },
    Unsupported,
}

pub fn detect_graphics_cap() -> GraphicsCapability {
    if is_multiplexer() {
        return GraphicsCapability::Unsupported;
    }
    if !is_kitty_capable_terminal() {
        return GraphicsCapability::Unsupported;
    }
    let (cw, ch) = query_cell_pixel_size().unwrap_or((8, 16));
    GraphicsCapability::Kitty {
        cell_width: cw,
        cell_height: ch,
    }
}

fn is_multiplexer() -> bool {
    std::env::var("TMUX").is_ok()
        || std::env::var("ZELLIJ").is_ok()
        || std::env::var("STY").is_ok()
        || std::env::var("TERM")
            .map(|t| t.starts_with("screen"))
            .unwrap_or(false)
}

fn is_kitty_capable_terminal() -> bool {
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        return matches!(term.as_str(), "WezTerm" | "ghostty");
    }
    false
}

#[cfg(unix)]
fn query_cell_pixel_size() -> Option<(u16, u16)> {
    use std::mem::MaybeUninit;
    let mut ws = MaybeUninit::<libc::winsize>::uninit();
    // SAFETY: ws is a valid MaybeUninit pointer; ioctl writes into it on success.
    let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, ws.as_mut_ptr()) };
    if ret != 0 {
        return None;
    }
    // SAFETY: ioctl returned 0, so ws is fully initialized.
    let ws = unsafe { ws.assume_init() };
    if ws.ws_col == 0 || ws.ws_row == 0 || ws.ws_xpixel == 0 || ws.ws_ypixel == 0 {
        return None;
    }
    Some((ws.ws_xpixel / ws.ws_col, ws.ws_ypixel / ws.ws_row))
}

#[cfg(not(unix))]
fn query_cell_pixel_size() -> Option<(u16, u16)> {
    None
}

impl GraphicsCapability {
    pub fn is_kitty(&self) -> bool {
        matches!(self, GraphicsCapability::Kitty { .. })
    }

    pub fn render_params(&self) -> Option<RenderParams> {
        match self {
            GraphicsCapability::Kitty {
                cell_width,
                cell_height,
            } => Some(RenderParams::from_cell_size(*cell_width, *cell_height)),
            GraphicsCapability::Unsupported => None,
        }
    }

    pub fn redetect_cell_size(&mut self) {
        if let GraphicsCapability::Kitty {
            ref mut cell_width,
            ref mut cell_height,
        } = self
        {
            if let Some((w, h)) = query_cell_pixel_size() {
                *cell_width = w;
                *cell_height = h;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn unsupported_when_tmux_set() {
        std::env::set_var("TMUX", "/tmp/tmux-1000/default,12345,0");
        let cap = detect_graphics_cap();
        assert!(matches!(cap, GraphicsCapability::Unsupported));
        std::env::remove_var("TMUX");
    }
}
