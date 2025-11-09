/// Mathematical utilities for the simulation

/// Linear interpolation
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamp a value between min and max
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Calculate a sigmoid curve (useful for utility functions)
pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Normalize a value from one range to another
pub fn normalize(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    let old_range = old_max - old_min;
    let new_range = new_max - new_min;
    (((value - old_min) * new_range) / old_range) + new_min
}

