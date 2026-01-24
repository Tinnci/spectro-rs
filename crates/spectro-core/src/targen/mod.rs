/// A single color patch definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub id: usize,
}

impl Patch {
    pub fn new(id: usize, r: f32, g: f32, b: f32) -> Self {
        Self { id, r, g, b }
    }
}

/// Helper to generate test patch sets
pub struct TargetGenerator;

impl TargetGenerator {
    /// Generate a ramp of gray steps.
    pub fn gray_ramp(steps: usize) -> Vec<Patch> {
        let mut patches = Vec::with_capacity(steps);
        for i in 0..steps {
            let v = i as f32 / (steps - 1) as f32;
            patches.push(Patch::new(i, v, v, v));
        }
        patches
    }

    /// Generate a simple RGB + Gray verification set.
    pub fn basic_verification() -> Vec<Patch> {
        vec![
            Patch::new(0, 0.0, 0.0, 0.0), // Black
            Patch::new(1, 1.0, 1.0, 1.0), // White
            Patch::new(2, 1.0, 0.0, 0.0), // Red
            Patch::new(3, 0.0, 1.0, 0.0), // Green
            Patch::new(4, 0.0, 0.0, 1.0), // Blue
            Patch::new(5, 0.0, 1.0, 1.0), // Cyan
            Patch::new(6, 1.0, 0.0, 1.0), // Magenta
            Patch::new(7, 1.0, 1.0, 0.0), // Yellow
        ]
    }

    /// Generate a grid-based cube (e.g. 3x3x3).
    pub fn grid(steps_per_channel: usize) -> Vec<Patch> {
        let mut patches = Vec::new();
        let div = (steps_per_channel - 1).max(1) as f32;
        let mut id = 0;

        for r in 0..steps_per_channel {
            for g in 0..steps_per_channel {
                for b in 0..steps_per_channel {
                    patches.push(Patch::new(
                        id,
                        r as f32 / div,
                        g as f32 / div,
                        b as f32 / div,
                    ));
                    id += 1;
                }
            }
        }
        patches
    }
}
