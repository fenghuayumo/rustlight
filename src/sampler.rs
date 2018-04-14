use cgmath::Point2;
// For random number
use rand;
use rand::distributions::{IndependentSample, Range};

// FIXME: This code is not used for now.
// FIXME: Found a way to make compatible with multi-threading
pub trait Sampler {
    fn next(&mut self) -> f32;
    fn next2d(&mut self) -> Point2<f32>;
}

pub trait SamplerMCMC {
    fn accept(&mut self);
    fn reject(&mut self);
}

pub struct IndependentSampler {
    rnd: rand::StdRng,
    dist: Range<f32>,
}

impl Sampler for IndependentSampler {
    fn next(&mut self) -> f32 {
        self.dist.ind_sample(&mut self.rnd)
    }
    fn next2d(&mut self) -> Point2<f32> {
        let x = self.dist.ind_sample(&mut self.rnd);
        let y = self.dist.ind_sample(&mut self.rnd);
        Point2::new(x, y)
    }
}

impl Default for IndependentSampler {
    fn default() -> IndependentSampler {
        IndependentSampler {
            rnd: rand::StdRng::new().unwrap(),
            dist: Range::new(0.0, 1.0),
        }
    }
}

pub trait Mutator {
    fn mutate(&self, v: f32, r: f32) -> f32;
}

struct MutatorKelemen {
    pub s1: f32,
    pub s2: f32,
    log_ratio: f32,
}

impl MutatorKelemen {
    pub fn new(s1: f32, s2: f32) -> Self {
        MutatorKelemen {
            s1,
            s2,
            log_ratio: (s2 / s1).log2(), // FIXME: Check if it is log2 or log10
        }
    }
}

impl Default for MutatorKelemen {
    fn default() -> Self {
        MutatorKelemen::new(
            1.0 / 1024.0,
            1.0 / 64.0,
        )
    }
}

impl Mutator for MutatorKelemen {
    fn mutate(&self, v: f32, r: f32) -> f32 {
        let (add, r) = if r < 0.5 { (true, r * 2.0) } else {(false, 2.0 * (r - 0.5))};
        let dv = self.s2 + (r * self.log_ratio);
        if add {
            let mut v = v + dv;
            if v > 1.0 { v -= 1.0 }
            v
        } else {
            let mut v = v - dv;
            if v < 0.0 { v += 1.0 }
            v
        }
    }
}

struct SampleReplayValue {
    pub value: f32,
    pub modify: usize,
}

pub struct IndependentSamplerReplay {
    rnd: rand::StdRng,
    dist: Range<f32>,
    values: Vec<SampleReplayValue>,
    backup: Vec<(usize, f32)>,
    mutator: Box<Mutator>,
    time: usize,
    time_large: usize,
    indice: usize,
    pub large_step: bool,
}

impl Sampler for IndependentSamplerReplay {
    fn next(&mut self) -> f32 {
        let i = self.indice;
        let v = self.sample(i);
        self.indice += 1;
        v
    }

    fn next2d(&mut self) -> Point2<f32> {
        let i1 = self.indice;
        let i2 = self.indice + 1;
        let v1 = self.sample(i1);
        let v2 = self.sample(i2);
        self.indice += 2;
        Point2::new(v1, v2)
    }
}

impl SamplerMCMC for IndependentSamplerReplay {
    fn accept(&mut self) {
        self.backup.clear();
        if self.large_step {
            self.time_large = self.time;
        }
        self.time += 1;
        self.indice += 1;
    }

    fn reject(&mut self) {
        for &(i, v) in &self.backup {
            self.values[i].value = v;
        }
        self.backup.clear();
        self.time += 1;
        self.indice = 0;
    }
}

impl Default for IndependentSamplerReplay {
    fn default() -> Self {
        IndependentSamplerReplay {
            rnd: rand::StdRng::new().unwrap(),
            dist: Range::new(0.0, 1.0),
            values: vec![],
            backup: vec![],
            mutator: Box::new(MutatorKelemen::default()),
            time: 0,
            time_large: 0,
            indice: 0,
            large_step: false,
        }
    }
}

//FIXME: Make not representable a sampler that are not accept
impl IndependentSamplerReplay {
    // Constructor to change the mutator technique
    fn mutator(mut self, mutator: Box<Mutator>) -> Self {
        self.mutator = mutator;
        self
    }

    fn sample(&mut self, i: usize) -> f32 {
        while i > self.values.len() {
            let value = self.rand();
            self.values.push(SampleReplayValue {
                value,
                modify: 0,
            })
        }

        if self.values[i].modify < self.time {
            if self.large_step {
                self.backup.push((i, self.values[i].value));
                let value = self.rand();
                self.values[i] = SampleReplayValue {
                    value,
                    modify: self.time,
                }
            } else {
                if self.values[i].modify < self.time_large {
                    let value = self.rand();
                    self.values[i] = SampleReplayValue {
                        value,
                        modify: self.time_large,
                    }
                }

                while self.values[i].modify + 1 < self.time {
                    let random = self.rand();
                    self.values[i].value = self.mutator.mutate(
                        self.values[i].value, random);
                    self.values[i].modify += 1;
                }

                self.backup.push((i, self.values[i].value));
                let random = self.rand();
                self.values[i] = SampleReplayValue {
                    value: self.mutator.mutate(
                        self.values[i].value,
                        random
                    ),
                    modify: self.time,
                }
            }
        }

        self.values[i].value
    }

    fn rand(&mut self) -> f32 {
        self.dist.ind_sample(&mut self.rnd)
    }
}