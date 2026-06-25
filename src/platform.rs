//! Platform-specific window/system integration.
//!
//! On macOS this reproduces the Electron overlay window behavior: transparent,
//! click-through, screen-saver level, visible on all spaces, and a dock-less
//! (menu-bar-only) application. On other platforms the functions are best-effort
//! no-ops so the rest of the app still builds and runs.

use std::ffi::c_void;

/// A display's bounds in screen coordinates `(x, y, width, height)`.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2::encode::{Encode, Encoding};
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

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
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
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
            let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
            if screens.is_null() {
                return out;
            }
            let count: usize = msg_send![screens, count];
            for i in 0..count {
                let screen: *mut AnyObject = msg_send![screens, objectAtIndex: i];
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

    /// Apply overlay semantics to a freshly opened window and move it to cover `screen_index`.
    pub fn configure_overlay(ns_view: *mut c_void, screen_index: usize) {
        if ns_view.is_null() {
            return;
        }
        unsafe {
            let view: *mut AnyObject = ns_view.cast();
            let window: *mut AnyObject = msg_send![view, window];
            if window.is_null() {
                return;
            }

            // Click-through + always-on-top + visible everywhere.
            let _: () = msg_send![window, setIgnoresMouseEvents: true];
            let _: () = msg_send![window, setLevel: SCREEN_SAVER_LEVEL];
            let _: () = msg_send![window, setCollectionBehavior: COLLECTION_BEHAVIOR];
            let _: () = msg_send![window, setOpaque: false];
            let _: () = msg_send![window, setHasShadow: false];

            let clear: *mut AnyObject = msg_send![class!(NSColor), clearColor];
            let _: () = msg_send![window, setBackgroundColor: clear];

            // Position/size to exactly cover the target display.
            let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
            if !screens.is_null() {
                let count: usize = msg_send![screens, count];
                if screen_index < count {
                    let screen: *mut AnyObject = msg_send![screens, objectAtIndex: screen_index];
                    let frame: CGRect = msg_send![screen, frame];
                    let _: () = msg_send![window, setFrame: frame, display: true];
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    pub fn hide_dock() {}

    pub fn screen_frames() -> Vec<ScreenFrame> {
        // Fallback: a single 1920x1080 display.
        vec![ScreenFrame {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        }]
    }

    pub fn configure_overlay(_ns_view: *mut c_void, _screen_index: usize) {}
}

pub use imp::{configure_overlay, hide_dock, screen_frames};
