use objc2::msg_send_id;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::MainThreadMarker;

use super::DisplayController;

pub struct NativeDisplay {
    // Background window (black curtain)
    bg_window: Retained<NSWindow>,
    // Patch window (actual color)
    patch_window: Retained<NSWindow>,
}

impl DisplayController for NativeDisplay {
    fn new() -> Result<Self, String> {
        // Ensure we are on the main thread for UI operations
        let mtm = MainThreadMarker::new()
            .ok_or("NativeDisplay must be initialized on the Main Thread.")?;

        // Get the main screen frame
        let screens = NSScreen::screens(mtm);
        let screen = screens.first().ok_or("No screen detected.")?;
        let frame = screen.frame();

        // NSBackingStoreBuffered = 2
        let backing_store = unsafe { std::mem::transmute::<usize, NSBackingStoreType>(2) };
        // NSScreenSaverWindowLevel = 1000 (roughly, or 2000+. 2500 is very high)
        // kCGScreenSaverWindowLevel is often used.
        // Let's use a standard high level.
        // NSWindowLevel is a type alias (NSInteger/i64)
        let level = 1000 as NSWindowLevel;

        unsafe {
            // 1. Create Background Window (Curtain)
            let style = NSWindowStyleMask::Borderless;
            let alloc_bg = mtm.alloc::<NSWindow>();
            let bg_window: Retained<NSWindow> = msg_send_id![
                alloc_bg,
                initWithContentRect: frame,
                styleMask: style,
                backing: backing_store,
                defer: false
            ];
            bg_window.setLevel(level);
            bg_window.setOpaque(true);
            bg_window.setHasShadow(false);
            bg_window.setIgnoresMouseEvents(true);

            // Set background to solid black
            let black = NSColor::colorWithDeviceRed_green_blue_alpha(0.0, 0.0, 0.0, 1.0);
            bg_window.setBackgroundColor(Some(&black));

            // 2. Create Patch Window (Centered)
            // Define patch size (e.g., 50% of screen or fixed size like 400x400)
            // Let's go with a reasonable fixed size for now or proportional.
            // ArgyllCMS typically uses a small window unless -f is specified.
            // Let's use 500x500 points centered.
            let patch_size = 500.0;
            let patch_rect = objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(
                    frame.origin.x + (frame.size.width - patch_size) / 2.0,
                    frame.origin.y + (frame.size.height - patch_size) / 2.0,
                ),
                objc2_foundation::NSSize::new(patch_size, patch_size),
            );

            let alloc_patch = mtm.alloc::<NSWindow>();
            let patch_window: Retained<NSWindow> = msg_send_id![
                alloc_patch,
                initWithContentRect: patch_rect,
                styleMask: style,
                backing: backing_store,
                defer: false
            ];
            // Patch needs to be above background. +1 level is enough.
            patch_window.setLevel((level + 1) as NSWindowLevel);
            patch_window.setOpaque(true);
            patch_window.setHasShadow(false);
            patch_window.setIgnoresMouseEvents(true);
            // Default to black initially
            patch_window.setBackgroundColor(Some(&black));

            // Show windows
            bg_window.makeKeyAndOrderFront(None);
            patch_window.makeKeyAndOrderFront(None);

            Ok(Self {
                bg_window,
                patch_window,
            })
        }
    }

    fn show_color(&self, r: f32, g: f32, b: f32) {
        unsafe {
            // Use device RGB to bypass color management
            let color =
                NSColor::colorWithDeviceRed_green_blue_alpha(r as f64, g as f64, b as f64, 1.0);
            self.patch_window.setBackgroundColor(Some(&color));

            // Force immediate flush (Argyll does this via [window display])
            self.patch_window.display();
        }
    }
}

impl NativeDisplay {
    /// Hide/Close the window.
    pub fn close(&self) {
        self.patch_window.close();
        self.bg_window.close();
    }
}

impl Drop for NativeDisplay {
    fn drop(&mut self) {
        self.close();
    }
}
