//! What a capture does to its samples as it is read, over the whole of it.
//!
//! An edited capture keeps its sources' own samples and applies one adjustment
//! on top: a spectral inversion and a level. Reading, saving and the minimap
//! all apply the same one, so what is shown is what a save writes.

/// A spectral inversion and a gain applied to every sample of a capture.
///
/// Inverting conjugates a complex sample (`Q` changes sign), which mirrors its
/// band about the reference frequency, and multiplies a real sample by
/// `(−1)^n`, which mirrors `0 .. Fs/2` about `Fs/4`. The gain scales I and Q
/// by one factor, so a complex sample keeps its phase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adjustment {
    invert: bool,
    gain: f64,
}

// The gain is finite by construction, so equality is total.
impl Eq for Adjustment {}

impl Default for Adjustment {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Adjustment {
    pub const IDENTITY: Self = Self {
        invert: false,
        gain: 1.0,
    };

    /// A adjustment, or none for a gain that is not a positive finite number.
    pub fn new(invert: bool, gain: f64) -> Option<Self> {
        (gain.is_finite() && gain > 0.0).then_some(Self { invert, gain })
    }

    pub const fn invert(self) -> bool {
        self.invert
    }

    pub const fn gain(self) -> f64 {
        self.gain
    }

    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// The same gain with the inversion switched.
    #[must_use]
    pub const fn inverted(self) -> Self {
        Self {
            invert: !self.invert,
            gain: self.gain,
        }
    }

    /// The same inversion with another gain, or none for a gain [`Adjustment::new`] refuses.
    pub fn with_gain(self, gain: f64) -> Option<Self> {
        Self::new(self.invert, gain)
    }

    /// What value `channel` of capture sample `index` is multiplied by.
    ///
    /// `channels` is 2 for a complex capture, whose second channel is Q, and 1 for a real one.
    pub fn factor(self, channels: usize, channel: usize, index: u64) -> f64 {
        let flipped = self.invert
            && match channels {
                1 => index % 2 == 1,
                _ => channel == 1,
            };
        if flipped { -self.gain } else { self.gain }
    }

    /// Adjustment interleaved values whose first sample is capture sample `first`.
    pub fn apply(self, values: &mut [f32], channels: usize, first: u64) {
        if self.is_identity() || channels == 0 {
            return;
        }
        for (offset, sample) in values.chunks_exact_mut(channels).enumerate() {
            let index = first + offset as u64;
            for (channel, value) in sample.iter_mut().enumerate() {
                *value = (f64::from(*value) * self.factor(channels, channel, index)) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    include!("adjustment_tests.rs");
}
