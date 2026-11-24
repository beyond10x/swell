// A fixture of libm-check: the methods an engine crate may call, and a forbidden call in a
// comment line, which the check skips. Not compiled.

pub fn allowed(x: f64, a: u32, b: u32) -> (f64, f64, u32) {
    // x.sin()
    let root = x.sqrt();
    let floor = x.floor();
    let least = a.min(b);
    (root, floor, least)
}
