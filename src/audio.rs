//! The audio sample stream: what the 2A03's AD1/AD2 taps hand to the
//! mixer. AUTHORED; the first consumer is the 2A03 repo's first-sound
//! milestone, and the mixer itself (the resistor network off the NES
//! schematic) lives in the console, not here.

/// A run of driver levels off AD1 and AD2, one pair per sample, with the
/// sample rate carried as an exact ratio (the family's rates are ratios
/// of the master crystal and rounding one into a float is how 60.0988
/// got conflated once already). Levels are in the units the producing
/// repo's transcription or measurement uses, stated by that repo's
/// report.
#[derive(Clone, Debug, Default)]
pub struct AudioSamples {
    /// Samples per second as numerator/denominator.
    pub rate_num: u64,
    pub rate_den: u64,
    pub ad1: Vec<f32>,
    pub ad2: Vec<f32>,
}

impl AudioSamples {
    pub fn new(rate_num: u64, rate_den: u64) -> AudioSamples {
        AudioSamples {
            rate_num,
            rate_den,
            ad1: Vec::new(),
            ad2: Vec::new(),
        }
    }

    pub fn push(&mut self, ad1: f32, ad2: f32) {
        self.ad1.push(ad1);
        self.ad2.push(ad2);
    }

    pub fn len(&self) -> usize {
        self.ad1.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ad1.is_empty()
    }
}
