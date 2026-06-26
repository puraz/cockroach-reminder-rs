//! Platform-specific window/system integration.
//!
//! On macOS this reproduces the Electron overlay window behavior: transparent,
//! click-through, screen-saver level, visible on all spaces, and a dock-less
//! (menu-bar-only) application.
//!
//! On Windows it uses Win32 [`EnumDisplayMonitors`] for multi-monitor detection
//! and ensures the overlay is topmost via [`SetWindowPos`].
//!
//! On Linux/X11 it uses `x11rb` to query Xinerama (multi-monitor) and sets the
//! `_NET_WM_STATE_ABOVE` EWMH property so the overlay stays on top.
//!
//! The non-macOS paths cannot produce click-through windows (the user accepted
//! mouse interception during breaks as intentional "forced rest" behaviour).

/// A display's bounds in screen coordinates `(x, y, width, height)`.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// ---------------------------------------------------------------------------
// macOS – AppKit / objc2
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::encode::{Encode, Encoding};
    use objc2::{class, msg_send};
    use raw_window_handle::RawWindowHandle;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding =
            Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    // Window level matching Electron's `setAlwaysOnTop('screen-saver')`.
    const SCREEN_SAVER_LEVEL: isize = 1000;
    // canJoinAllSpaces (1) | stationary (16) | fullScreenAuxiliary (256)
    const COLLECTION_BEHAVIOR: usize = 1 | 16 | 256;
    // NSApplicationActivationPolicyAccessory
    const ACTIVATION_ACCESSORY: isize = 1;

    /// Hide the dock icon — make this a menu-bar-only (accessory) app.
    pub fn hide_dock() {
        unsafe {
            let app: *mut objc2::runtime::AnyObject =
                msg_send![class!(NSApplication), sharedApplication];
            if !app.is_null() {
                // -[NSApplication setActivationPolicy:] returns BOOL.
                let _: bool = msg_send![app, setActivationPolicy: ACTIVATION_ACCESSORY];
            }
        }
    }


    /// Enumerate all displays.
    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut out = Vec::new();
        unsafe {
            let screens: *mut objc2::runtime::AnyObject = msg_send![class!(NSScreen), screens];
            if screens.is_null() {
                return out;
            }
            let count: usize = msg_send![screens, count];
            for i in 0..count {
                let screen: *mut objc2::runtime::AnyObject = msg_send![screens, objectAtIndex: i];
                let frame: CGRect = msg_send![screen, frame];
                out.push(ScreenFrame {
                    x: frame.origin.x,
                    y: frame.origin.y,
                    width: frame.size.width,
                    height: frame.size.height,
                });
            }
        }
        out
    }

    /// Apply overlay semantics to a freshly opened window.
    pub fn configure_overlay(handle: &RawWindowHandle, screen_index: usize) {
        let ns_view = match handle {
            RawWindowHandle::AppKit(h) => h.ns_view.as_ptr(),
            _ => return,
        };
        if ns_view.is_null() {
            return;
        }
        unsafe {
            let view: *mut objc2::runtime::AnyObject = ns_view.cast();
            let window: *mut objc2::runtime::AnyObject = msg_send![view, window];
            if window.is_null() {
                return;
            }

            // Click-through + always-on-top + visible everywhere.
            let _: () = msg_send![window, setIgnoresMouseEvents: true];
            let _: () = msg_send![window, setLevel: SCREEN_SAVER_LEVEL];
            let _: () = msg_send![window, setCollectionBehavior: COLLECTION_BEHAVIOR];
            let _: () = msg_send![window, setOpaque: false];
            let _: () = msg_send![window, setHasShadow: false];

            let clear: *mut objc2::runtime::AnyObject = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![window, setBackgroundColor: clear];

            // Position/size to exactly cover the target display.
            let screens: *mut objc2::runtime::AnyObject = msg_send![class!(NSScreen), screens];
            if !screens.is_null() {
                let count: usize = msg_send![screens, count];
                if screen_index < count {
                    let screen: *mut objc2::runtime::AnyObject =
                        msg_send![screens, objectAtIndex: screen_index];
                    let frame: CGRect = msg_send![screen, frame];
                    let _: () = msg_send![window, setFrame: frame, display: true];
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Windows – Win32 API
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC};
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos;
    use windows_sys::Win32::UI::WindowsAndMessaging::{HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE};

    pub fn hide_dock() {}

    unsafe extern "system" fn monitor_enum_proc(
        _hmonitor: HDC,
        _hdc: HDC,
        rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let frames = &mut *(data as *mut Vec<ScreenFrame>);
        frames.push(ScreenFrame {
            x: (*rect).left as f64,
            y: (*rect).top as f64,
            width: ((*rect).right - (*rect).left) as f64,
            height: ((*rect).bottom - (*rect).top) as f64,
        });
        TRUE
    }

    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut frames = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(monitor_enum_proc),
                &mut frames as *mut _ as isize,
            );
        }
        frames
    }

    pub fn configure_overlay(handle: &RawWindowHandle, _screen_index: usize) {
        let hwnd = match handle {
            RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
            _ => return,
        };
        unsafe {
            // Ensure the window is topmost (belt-and-suspenders; winit usually
            // sets WS_EX_TOPMOST for AlwaysOnTop, but this guarantees it).
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Linux (X11) – x11rb
// ---------------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use raw_window_handle::RawWindowHandle;
    use x11rb::connection::Connection;
    use x11rb::protocol::xinerama::ConnectionExt as _;
    use x11rb::protocol::xproto::ConnectionExt as _;
    pub fn hide_dock() {}
    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut frames = Vec::new();
        let (conn, _screen_num) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(_) => return frames,
        };

        // Xinerama multi-monitor query (cookie → reply chain).
        if let Ok(cookie) = conn.xinerama_is_active() {
            if let Ok(reply) = cookie.reply() {
                if reply.state != 0 {
                    if let Ok(cookie) = conn.xinerama_query_screens() {
                        if let Ok(reply) = cookie.reply() {
                            for si in reply.screen_info {
                                frames.push(ScreenFrame {
                                    x: si.x_org as f64,
                                    y: si.y_org as f64,
                                    width: si.width as f64,
                                    height: si.height as f64,
                                });
                            }
                        }
                    }
                }
            }
        }

        if frames.is_empty() {
            // Fallback: use the root window geometry.
            let setup = conn.setup();
            if let Some(screen) = setup.roots.get(_screen_num) {
                frames.push(ScreenFrame {
                    x: 0.0,
                    y: 0.0,
                    width: screen.width_in_pixels as f64,
                    height: screen.height_in_pixels as f64,
                });
            }
        }

        frames
    }

    pub fn configure_overlay(handle: &RawWindowHandle, _screen_index: usize) {
        let window = match handle {
            RawWindowHandle::Xlib(h) => h.window as u32,
            RawWindowHandle::Xcb(_) => {
                return;
            }
            _ => return,
        };

        // Send _NET_WM_STATE ClientMessage to the ROOT window so the WM sees it.
        let (conn, _) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(_) => return,
        };

        let setup = conn.setup();
        let screen_idx = _screen_index.min(setup.roots.len().saturating_sub(1));
        let root = setup.roots[screen_idx].root;

        // Intern atoms (inline, no closure – avoids complex type inference).
        let state_atom = match conn.intern_atom(false, b"_NET_WM_STATE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(_) => return,
            },
            Err(_) => return,
        };
        let above_atom = match conn.intern_atom(false, b"_NET_WM_STATE_ABOVE") {
            Ok(c) => match c.reply() {
                Ok(r) => r.atom,
                Err(_) => return,
            },
            Err(_) => return,
        };

        // ClientMessageEvent::new(format, window, type_, data)
        // data is [u32; 5] which implements Into<ClientMessageData>.
        let event = x11rb::protocol::xproto::ClientMessageEvent::new(
            32,
            window,
            state_atom,
            [2u32, above_atom, 0, 0, 0], // _NET_WM_STATE_ADD
        );
        let _ = conn.send_event(
            false,
            root,
            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
        let _ = conn.flush();
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub use imp::{configure_overlay, hide_dock, screen_frames};
