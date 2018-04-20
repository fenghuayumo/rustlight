use sampler::*;
use scene::*;

pub trait Integrator<T>: Sync + Send {
    fn compute<S: Sampler>(&self, pix: (u32, u32), scene: &Scene, sampler: &mut S) -> T;
}

/// Power heuristic for path tracing or direct lighting
fn mis_weight(pdf_a: f32, pdf_b: f32) -> f32 {
    if pdf_a == 0.0 {
        warn!("MIS weight requested for 0.0 pdf");
        return 0.0;
    }
    assert!(pdf_a.is_finite());
    assert!(pdf_b.is_finite());
    let w = pdf_a.powi(2) / (pdf_a.powi(2) + pdf_b.powi(2));
    if w.is_finite() {
        w
    } else {
        warn!("Not finite MIS weight for: {} and {}", pdf_a, pdf_b);
        0.0
    }
}

pub mod ao;
pub mod direct;
pub mod path;
pub mod path_explicit;
pub mod gradient_domain;
pub mod prelude;