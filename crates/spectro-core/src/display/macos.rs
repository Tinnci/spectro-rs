use objc2::msg_send_id;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSScreen, NSWindow, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::MainThreadMarker;

use super::DisplayController;

pub struct NativeDisplay {
    // Keep window alive
    window: Retained<NSWindow>,
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
            // Create a borderless window covering the full screen
            // Use msg_send_id! for robust init calling
            let style = NSWindowStyleMask::Borderless;

            let alloc = mtm.alloc::<NSWindow>();
            let window: Retained<NSWindow> = msg_send_id![
                alloc,
                initWithContentRect: frame,
                styleMask: style,
                backing: backing_store,
                defer: false
            ];
            // Set window level
            window.setLevel(level);

            // Configure window properties
            window.setOpaque(true);
            window.setHasShadow(false);
            window.setIgnoresMouseEvents(true);

            // Set initial color to Black
            let black = NSColor::colorWithDeviceRed_green_blue_alpha(0.0, 0.0, 0.0, 1.0);
            window.setBackgroundColor(Some(&black));

            // Show the window
            window.makeKeyAndOrderFront(None);

            Ok(Self { window })
        }
    }

    fn show_color(&self, r: f32, g: f32, b: f32) {
        unsafe {
            // Use device RGB to bypass color management
            let color =
                NSColor::colorWithDeviceRed_green_blue_alpha(r as f64, g as f64, b as f64, 1.0);
            self.window.setBackgroundColor(Some(&color));

            // Force immediate flush (Argyll does this via [window display])
            self.window.display();
        }
    }
}

impl NativeDisplay {
    /// Hide/Close the window.
    pub fn close(&self) {
        self.window.close();
    }
}

impl Drop for NativeDisplay {
    fn drop(&mut self) {
        self.close();
    }
}
