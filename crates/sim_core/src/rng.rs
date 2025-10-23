//! Deterministic random stream utilities.
//!
//! Each [`Stream`] instance represents an independent pseudo-random sequence
//! derived from the simulation seed, a logical stage label, and the current
//! tick. Substreams can be derived deterministically without mutating the
//! parent stream, which allows kernels to spawn region-level RNGs while
//! preserving reproducibility.

#[derive(Clone, Debug)]
pub struct Stream {
    /// Upper 64 bits store the logical stream id; lower 64 bits store the
    /// rolling counter for splitmix-style generation.
    state: u128,
}

impl Stream {
    /// Construct a stream for the given `(seed, stage, tick)` triple.
    pub fn from(seed: u64, stage: &str, tick: u64) -> Self {
        let stage_hash = fnv1a64(stage.as_bytes());
        let mut stream_id = seed
            .wrapping_mul(0xA0761D6478BD642F)
            .wrapping_add(0xE7037ED1A0B428DB)
            ^ tick.wrapping_mul(0x8E9D5A8F6A09E667)
            ^ stage_hash;
        stream_id = mix64(stream_id);
        let counter = mix64(stream_id ^ 0xD1342543DE82EF95);
        Self {
            state: (u128::from(stream_id) << 64) | u128::from(counter),
        }
    }

    /// Deterministically derive a child stream identified by `label`.
    pub fn derive(&self, label: u64) -> Self {
        let parent_id = (self.state >> 64) as u64;
        let derived = mix64(parent_id ^ mix64(label ^ 0x94D049BB133111EB));
        let counter = mix64(derived ^ 0xBF58476D1CE4E5B9);
        Self {
            state: (u128::from(derived) << 64) | u128::from(counter),
        }
    }

    /// Advance the stream and return the next `u64` sample.
    pub fn next_u64(&mut self) -> u64 {
        let stream_id = (self.state >> 64) as u64;
        let mut counter = self.state as u64;
        counter = counter.wrapping_add(0x9E3779B97F4A7C15);
        self.state = (u128::from(stream_id) << 64) | u128::from(counter);
        mix64(stream_id ^ counter)
    }

    /// Advance the stream and return the next `f32` sample in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        const SCALE: f32 = (1u32 << 24) as f32;
        ((self.next_u64() >> 40) as f32) / SCALE
    }

    /// Advance the stream and return the next `f64` sample in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = (1u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / SCALE
    }

    /// Advance the stream and return the next `f64` sample in `[-1, 1)`.
    pub fn next_signed_unit(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }
}

/// Produce a deterministic label for deriving child streams.
pub fn stream_label(name: &str) -> u64 {
    fnv1a64(name.as_bytes())
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::Stream;

    #[test]
    fn derive_is_deterministic() {
        let base = Stream::from(42, "stage", 7);
        let mut derived_a = base.derive(5);
        let mut derived_b = base.derive(5);
        assert_eq!(derived_a.next_u64(), derived_b.next_u64());
        assert_eq!(derived_a.next_f64(), derived_b.next_f64());
    }

    #[test]
    fn stage_changes_stream() {
        let mut climate = Stream::from(1, "climate", 10);
        let mut ecology = Stream::from(1, "ecology", 10);
        assert_ne!(climate.next_u64(), ecology.next_u64());
    }
}
