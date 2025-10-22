/// Deterministic SplitMix64 RNG implementation providing reproducible substreams.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = (1u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / SCALE
    }

    pub fn next_signed_unit(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }
}

/// Project-level RNG that can spawn deterministic stage substreams.
#[derive(Clone, Debug)]
pub struct ProjectRng {
    seed: u64,
}

impl ProjectRng {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn stage(&self, stage: Stage, tick: u64) -> StageRng {
        let mut mix = SplitMix64::new(self.seed ^ stage.id());
        mix.state ^= tick;
        StageRng {
            stream: mix,
            counter: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Stage {
    Climate,
    Ecology,
}

impl Stage {
    fn id(self) -> u64 {
        match self {
            Stage::Climate => 0xC1_1A7E_u64,
            Stage::Ecology => 0xEC_0810_u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StageRng {
    stream: SplitMix64,
    counter: u64,
}

impl StageRng {
    pub fn next_f64(&mut self) -> f64 {
        self.counter += 1;
        self.stream.next_f64()
    }

    pub fn next_signed_unit(&mut self) -> f64 {
        self.counter += 1;
        self.stream.next_signed_unit()
    }

    pub fn fork_region(&self, region_index: usize) -> RegionRng {
        let seed = self.stream.clone();
        let mut mix = SplitMix64::new(seed.state ^ (region_index as u64 + 0x9E37));
        mix.state = mix.next_u64();
        RegionRng {
            stream: mix,
            counter: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegionRng {
    stream: SplitMix64,
    counter: u64,
}

impl RegionRng {
    pub fn next_f64(&mut self) -> f64 {
        self.counter += 1;
        self.stream.next_f64()
    }

    pub fn next_signed_unit(&mut self) -> f64 {
        self.counter += 1;
        self.stream.next_signed_unit()
    }
}
