// A fixture of libm-check: five forbidden calls, one per line. Not compiled.

pub fn forbidden(x: f64, a: f64, b: f64) -> f64 {
    let s = x.sin();
    let e = f64::exp(x);
    let m = x.mul_add(a, b);
    s + e + m
}

unsafe extern "C" {
    fn cos(x: f64) -> f64;
}

pub fn simd() {
    let _ = core::arch::x86_64::_mm_add_pd;
}
