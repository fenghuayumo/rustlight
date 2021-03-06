use crate::bsdfs::*;

pub struct BSDFBlend {
    pub bsdf1: Box<dyn BSDF + Sync + Send>,
    pub bsdf2: Box<dyn BSDF + Sync + Send>,
}

impl BSDF for BSDFBlend {
    fn sample(
        &self,
        uv: &Option<Vector2<f32>>,
        d_in: &Vector3<f32>,
        sample: Point2<f32>,
    ) -> Option<SampledDirection> {
        assert!(!self.bsdf1.is_smooth() && !self.bsdf2.is_smooth());

        let sampled_dir = if sample.x < 0.5 {
            let scaled_sample = Point2::new(sample.x * 2.0, sample.y);
            self.bsdf1.sample(uv, d_in, scaled_sample)
        } else {
            let scaled_sample = Point2::new((sample.x - 0.5) * 2.0, sample.y);
            self.bsdf2.sample(uv, d_in, scaled_sample)
        };

        if let Some(mut sampled_dir) = sampled_dir {
            sampled_dir.pdf = self.pdf(uv, d_in, &sampled_dir.d, Domain::SolidAngle);
            if sampled_dir.pdf.value() == 0.0 {
                None
            } else {
                sampled_dir.weight = self.eval(uv, d_in, &sampled_dir.d, Domain::SolidAngle)
                    / sampled_dir.pdf.value();
                Some(sampled_dir)
            }
        } else {
            None
        }
    }

    fn pdf(
        &self,
        uv: &Option<Vector2<f32>>,
        d_in: &Vector3<f32>,
        d_out: &Vector3<f32>,
        domain: Domain,
    ) -> PDF {
        let pdf_1 = self.bsdf1.pdf(uv, d_in, d_out, domain);
        let pdf_2 = self.bsdf2.pdf(uv, d_in, d_out, domain);
        if let (PDF::SolidAngle(pdf_1), PDF::SolidAngle(pdf_2)) = (pdf_1, pdf_2) {
            PDF::SolidAngle((pdf_1 + pdf_2) * 0.5)
        } else {
            panic!("get wrong type of BSDF");
        }
    }

    fn eval(
        &self,
        uv: &Option<Vector2<f32>>,
        d_in: &Vector3<f32>,
        d_out: &Vector3<f32>,
        domain: Domain,
    ) -> Color {
        self.bsdf1.eval(uv, d_in, d_out, domain) + self.bsdf2.eval(uv, d_in, d_out, domain)
    }

    fn roughness(&self, uv: &Option<Vector2<f32>>) -> f32 {
        // TODO: Use a more finer scheme when multiple component
        // BSDF will be implemented
        self.bsdf1.roughness(uv).min(self.bsdf2.roughness(uv))
    }

    fn is_smooth(&self) -> bool {
        if self.bsdf1.is_smooth() || self.bsdf2.is_smooth() {
            panic!("is smooth on blend material");
        }
        false
    }

    fn is_twosided(&self) -> bool {
        if !self.bsdf1.is_twosided() || !self.bsdf2.is_twosided() {
            panic!("is twosided on blend material");
        }
        true
    }
}
