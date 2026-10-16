//! Biquad filter design (the RBJ "Audio EQ Cookbook" formulas, which is also
//! what Equalizer APO and QuantumENGINE use) and frequency response, so the
//! service can hand PipeWire raw coefficients and work out how much headroom a
//! curve needs.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    Bell,
    LowShelf,
    HighShelf,
    /// Cuts below `freq`; `gain_db` is ignored.
    HighPass,
    /// Cuts above `freq`; `gain_db` is ignored.
    LowPass,
}

impl FilterType {
    pub const ALL: [FilterType; 5] =
        [FilterType::Bell, FilterType::LowShelf, FilterType::HighShelf, FilterType::HighPass, FilterType::LowPass];

    /// The name used on D-Bus and in settings: `bell`, `lowshelf`,
    /// `highshelf`, `highpass` or `lowpass`.
    pub fn as_str(self) -> &'static str {
        match self {
            FilterType::Bell => "bell",
            FilterType::LowShelf => "lowshelf",
            FilterType::HighShelf => "highshelf",
            FilterType::HighPass => "highpass",
            FilterType::LowPass => "lowpass",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }

    /// Whether the filter has a gain at all (pass filters only cut).
    pub fn has_gain(self) -> bool {
        !matches!(self, FilterType::HighPass | FilterType::LowPass)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub kind: FilterType,
    pub freq: f64,
    pub gain_db: f64,
    pub q: f64,
}

impl Band {
    pub const fn bell(freq: f64, gain_db: f64, q: f64) -> Self {
        Self { kind: FilterType::Bell, freq, gain_db, q }
    }
}

/// Coefficients normalised so that `a0` is 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coefficients {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Coefficients {
    /// Passes the signal through unchanged.
    pub const IDENTITY: Self = Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    pub fn design(band: &Band, rate: f64) -> Self {
        let w0 = 2.0 * PI * band.freq / rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * band.q);
        let a = 10f64.powf(band.gain_db / 40.0);
        let sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        let (b0, b1, b2, a0, a1, a2) = match band.kind {
            FilterType::Bell => (
                1.0 + alpha * a,
                -2.0 * cos,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos,
                1.0 - alpha / a,
            ),
            FilterType::LowShelf => (
                a * ((a + 1.0) - (a - 1.0) * cos + sqrt_a_alpha),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - sqrt_a_alpha),
                (a + 1.0) + (a - 1.0) * cos + sqrt_a_alpha,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - sqrt_a_alpha,
            ),
            FilterType::HighShelf => (
                a * ((a + 1.0) + (a - 1.0) * cos + sqrt_a_alpha),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - sqrt_a_alpha),
                (a + 1.0) - (a - 1.0) * cos + sqrt_a_alpha,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - sqrt_a_alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos) / 2.0,
                -(1.0 + cos),
                (1.0 + cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            FilterType::LowPass => (
                (1.0 - cos) / 2.0,
                1.0 - cos,
                (1.0 - cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
        };
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    /// The same filter with its output multiplied by `gain` (linear).
    pub fn scaled(self, gain: f64) -> Self {
        Self { b0: self.b0 * gain, b1: self.b1 * gain, b2: self.b2 * gain, ..self }
    }

    /// Gain at `freq`, in dB.
    pub fn magnitude_db(&self, freq: f64, rate: f64) -> f64 {
        let w = 2.0 * PI * freq / rate;
        // H(e^jw) = (b0 + b1 e^-jw + b2 e^-2jw) / (1 + a1 e^-jw + a2 e^-2jw)
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let num_re = self.b0 + self.b1 * c1 + self.b2 * c2;
        let num_im = -(self.b1 * s1 + self.b2 * s2);
        let den_re = 1.0 + self.a1 * c1 + self.a2 * c2;
        let den_im = -(self.a1 * s1 + self.a2 * s2);
        let num = num_re * num_re + num_im * num_im;
        let den = den_re * den_re + den_im * den_im;
        10.0 * (num / den).log10()
    }
}

/// Gain of a chain of filters at `freq`, in dB.
pub fn response_db(chain: &[Coefficients], freq: f64, rate: f64) -> f64 {
    chain.iter().map(|c| c.magnitude_db(freq, rate)).sum()
}

/// The highest gain the chain reaches anywhere in the audible range, in dB.
pub fn peak_db(chain: &[Coefficients], rate: f64) -> f64 {
    const POINTS: usize = 1024;
    let (lo, hi) = (20f64.ln(), 20_000f64.min(rate / 2.0 * 0.999).ln());
    (0..POINTS)
        .map(|i| (lo + (hi - lo) * i as f64 / (POINTS - 1) as f64).exp())
        .map(|f| response_db(chain, f, rate))
        .fold(f64::NEG_INFINITY, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn identity_is_flat() {
        for f in [20.0, 1_000.0, 19_000.0] {
            assert!(close(Coefficients::IDENTITY.magnitude_db(f, RATE), 0.0, 1e-9));
        }
    }

    #[test]
    fn pass_filters_are_3_db_down_at_their_corner_and_pass_the_other_side() {
        let hp = Coefficients::design(&Band { kind: FilterType::HighPass, freq: 100.0, gain_db: 0.0, q: std::f64::consts::FRAC_1_SQRT_2 }, RATE);
        assert!(close(hp.magnitude_db(100.0, RATE), -3.01, 0.02));
        assert!(close(hp.magnitude_db(5_000.0, RATE), 0.0, 0.01));
        assert!(hp.magnitude_db(25.0, RATE) < -20.0);
        let lp = Coefficients::design(&Band { kind: FilterType::LowPass, freq: 8_000.0, gain_db: 0.0, q: std::f64::consts::FRAC_1_SQRT_2 }, RATE);
        assert!(close(lp.magnitude_db(8_000.0, RATE), -3.01, 0.05));
        assert!(close(lp.magnitude_db(100.0, RATE), 0.0, 0.01));
    }

    #[test]
    fn filter_types_round_trip_through_their_names() {
        for t in FilterType::ALL {
            assert_eq!(FilterType::parse(t.as_str()), Some(t));
        }
    }

    #[test]
    fn bell_reaches_its_gain_at_the_centre() {
        let c = Coefficients::design(&Band::bell(1_000.0, 6.0, 1.0), RATE);
        assert!(close(c.magnitude_db(1_000.0, RATE), 6.0, 1e-6));
        assert!(close(c.magnitude_db(20.0, RATE), 0.0, 0.1));
        let cut = Coefficients::design(&Band::bell(1_000.0, -12.0, 3.5), RATE);
        assert!(close(cut.magnitude_db(1_000.0, RATE), -12.0, 1e-6));
    }

    #[test]
    fn shelves_settle_at_their_gain_on_the_shelved_side() {
        let low = Band { kind: FilterType::LowShelf, freq: 300.0, gain_db: -2.0, q: 1.0 };
        let c = Coefficients::design(&low, RATE);
        assert!(close(c.magnitude_db(20.0, RATE), -2.0, 0.05));
        assert!(close(c.magnitude_db(10_000.0, RATE), 0.0, 0.05));

        let high = Band { kind: FilterType::HighShelf, freq: 4_000.0, gain_db: -2.0, q: std::f64::consts::FRAC_1_SQRT_2 };
        let c = Coefficients::design(&high, RATE);
        assert!(close(c.magnitude_db(20_000.0, RATE), -2.0, 0.1));
        assert!(close(c.magnitude_db(50.0, RATE), 0.0, 0.05));
    }

    #[test]
    fn scaling_shifts_the_whole_curve() {
        let half = Coefficients::IDENTITY.scaled(0.5);
        assert!(close(half.magnitude_db(1_000.0, RATE), -6.0206, 1e-3));
    }

    #[test]
    fn peak_finds_the_largest_boost() {
        let chain = [
            Coefficients::design(&Band::bell(100.0, 3.0, 1.0), RATE),
            Coefficients::design(&Band::bell(8_000.0, 5.0, 2.0), RATE),
        ];
        assert!(close(peak_db(&chain, RATE), 5.0, 0.05));
        assert!(peak_db(&[Coefficients::IDENTITY], RATE).abs() < 1e-9);
    }
}
