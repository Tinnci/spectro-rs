use super::GammaController;
use core_graphics::display::{CGDirectDisplayID, CGMainDisplayID};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGDisplaySetTransferByTable(
        display: CGDirectDisplayID,
        tableSize: u32,
        redTable: *const f32,
        greenTable: *const f32,
        blueTable: *const f32,
    ) -> i32;
}

pub struct VcgtController {
    display_id: CGDirectDisplayID,
}

impl VcgtController {
    pub fn new() -> Result<Self, String> {
        unsafe {
            // Target the main display for now
            let id = CGMainDisplayID();
            Ok(Self { display_id: id })
        }
    }
}

impl GammaController for VcgtController {
    fn set_gamma_tables(&self, red: &[f32], green: &[f32], blue: &[f32]) -> Result<(), String> {
        let count = red.len() as u32;
        if count != green.len() as u32 || count != blue.len() as u32 {
            return Err("Channel lengths mismatch".into());
        }

        let res = unsafe {
            CGDisplaySetTransferByTable(
                self.display_id,
                count,
                red.as_ptr(),
                green.as_ptr(),
                blue.as_ptr(),
            )
        };

        if res == 0 {
            // kCGErrorSuccess is 0
            Ok(())
        } else {
            Err(format!("CGDisplaySetTransferByTable failed: {}", res))
        }
    }
}
