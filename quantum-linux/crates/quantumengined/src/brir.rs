//! Builds the impulse responses virtual surround plays through: for each
//! 7.1 speaker, what each ear hears from it in a small room (a binaural room
//! impulse response, BRIR).
//!
//! An HRTF measured in an anechoic chamber, played as-is, sounds inside the
//! head, dry, and coloured by the dummy head's own ears and microphones.
//! Harman's HX3D avoids that with post-processed HRIRs that have a short room
//! baked in, plus an ambience stage. This builds the same kind of thing from
//! open data and a little acoustics, with nothing of theirs:
//!
//! * the direct sound, from the MIT KEMAR measurements libmysofa ships;
//! * early reflections from a simulated shoebox room (image sources up to
//!   second order), each placed with its own HRTF and darkened a little per
//!   bounce;
//! * a short late tail of decaying noise, different in each ear, which is
//!   what moves sound out of the head;
//! * diffuse-field equalisation: the HRTF's average response over every
//!   direction is divided out, so only what *differs* between directions
//!   remains, and the dummy's own colouring goes.
//!
//! The result is written as a 14-channel float WAV (left and right ear for
//! each of the seven speakers, in [`super::surround`]'s order) for PipeWire's
//! convolver to play.

use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;
use std::ffi::CString;
use std::io::Write as _;
use std::path::Path;

pub const RATE: u32 = 48_000;
/// 256 ms: the room tail is about 50 dB down by then.
pub const LENGTH: usize = 12_288;

/// The room: a living-room-sized box, with the listener off-centre so front
/// and back don't mirror each other.
const ROOM: [f64; 3] = [6.0, 5.0, 3.0];
const LISTENER: [f64; 3] = [2.6, 2.4, 1.2];
/// How far the virtual speakers are from the listener, in metres.
const SPEAKER_DISTANCE: f64 = 2.0;
/// Pressure reflected per bounce (absorption 1 - 0.45² ≈ 80%): a furnished
/// room, so positions stay sharp enough for games.
const WALL_REFLECTION: f64 = 0.45;
const SPEED_OF_SOUND: f64 = 343.0;
/// Late tail: decay time to -60 dB, where it starts, and its energy relative
/// to the direct sound.
const RT60: f64 = 0.28;
const TAIL_START: f64 = 0.018;
const TAIL_ENERGY: f64 = 0.10;
/// Diffuse-field correction limits, in dB, and the band it works in; outside
/// it the correction is held flat.
const EQ_MAX_CUT: f64 = -18.0;
const EQ_MAX_BOOST: f64 = 6.0;
const EQ_BAND: (f64, f64) = (120.0, 14_000.0);
const EQ_TAPS: usize = 512;
const FFT_SIZE: usize = 4096;

mod ffi {
    use std::os::raw::{c_char, c_float, c_int};

    #[repr(C)]
    pub struct MysofaEasy {
        _private: [u8; 0],
    }

    #[link(name = "mysofa")]
    extern "C" {
        pub fn mysofa_open(
            filename: *const c_char,
            samplerate: c_float,
            filterlength: *mut c_int,
            err: *mut c_int,
        ) -> *mut MysofaEasy;
        pub fn mysofa_getfilter_float(
            easy: *mut MysofaEasy,
            x: c_float,
            y: c_float,
            z: c_float,
            left: *mut c_float,
            right: *mut c_float,
            delay_left: *mut c_float,
            delay_right: *mut c_float,
        );
        pub fn mysofa_close(easy: *mut MysofaEasy);
    }
}

/// An open SOFA HRTF set, resampled to [`RATE`] by libmysofa.
pub struct Hrtf {
    easy: *mut ffi::MysofaEasy,
    len: usize,
}

impl Hrtf {
    pub fn open(path: &Path) -> Result<Self, String> {
        let c_path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| "bad HRTF path".to_string())?;
        let (mut len, mut err) = (0, 0);
        // SAFETY: a valid C string and out-pointers; libmysofa owns the result
        // until mysofa_close in Drop.
        let easy = unsafe { ffi::mysofa_open(c_path.as_ptr(), RATE as f32, &mut len, &mut err) };
        if easy.is_null() || err != 0 || len <= 0 {
            return Err(format!("cannot read the HRTF {} (libmysofa error {err})", path.display()));
        }
        Ok(Self { easy, len: len as usize })
    }

    /// What the left and right ear hear from a source in direction `az`,
    /// `el` (degrees; azimuth anticlockwise from straight ahead, so 90 is
    /// left, as SOFA files measure it), with any separate delays applied.
    pub fn pair(&self, az: f64, el: f64) -> (Vec<f32>, Vec<f32>) {
        let (a, e) = (az.to_radians(), el.to_radians());
        let (x, y, z) = (e.cos() * a.cos(), e.cos() * a.sin(), e.sin());
        let mut left = vec![0f32; self.len];
        let mut right = vec![0f32; self.len];
        let (mut dl, mut dr) = (0f32, 0f32);
        // SAFETY: both buffers hold the filter length libmysofa reported.
        unsafe {
            ffi::mysofa_getfilter_float(
                self.easy,
                x as f32,
                y as f32,
                z as f32,
                left.as_mut_ptr(),
                right.as_mut_ptr(),
                &mut dl,
                &mut dr,
            );
        }
        // SOFA delays are in samples (here at RATE); most sets bake the ITD
        // into the responses and leave these at zero.
        (delayed(left, dl), delayed(right, dr))
    }
}

impl Drop for Hrtf {
    fn drop(&mut self) {
        // SAFETY: opened by mysofa_open and closed exactly once.
        unsafe { ffi::mysofa_close(self.easy) }
    }
}

fn delayed(ir: Vec<f32>, delay: f32) -> Vec<f32> {
    let n = delay.round().clamp(0.0, 256.0) as usize;
    if n == 0 {
        return ir;
    }
    let mut out = vec![0f32; n];
    out.extend(ir);
    out
}

/// One speaker's binaural room impulse response, left and right ear.
pub struct Brir {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

/// Builds the BRIRs for speakers at the given azimuths (degrees, SOFA
/// convention) from the HRTF set at `hrtf_path`.
pub fn build(hrtf_path: &Path, azimuths: &[f64]) -> Result<Vec<Brir>, String> {
    let hrtf = Hrtf::open(hrtf_path)?;
    let eq = diffuse_field_inverse(&hrtf);
    let mut brirs: Vec<Brir> =
        azimuths.iter().enumerate().map(|(i, &az)| room_response(&hrtf, az, 0x9E37_79B9 ^ (i as u64 + 1))).collect();
    for b in &mut brirs {
        b.left = convolve(&b.left, &eq);
        b.right = convolve(&b.right, &eq);
    }
    Ok(brirs)
}

/// The direct sound, the early reflections and the late tail for a speaker
/// at `az`, `SPEAKER_DISTANCE` away at ear height.
fn room_response(hrtf: &Hrtf, az: f64, seed: u64) -> Brir {
    let mut left = vec![0f32; LENGTH];
    let mut right = vec![0f32; LENGTH];
    let a = az.to_radians();
    let source = [
        LISTENER[0] + SPEAKER_DISTANCE * a.cos(),
        LISTENER[1] + SPEAKER_DISTANCE * a.sin(),
        LISTENER[2],
    ];

    let mut direct_energy = 0.0;
    for (position, order) in image_sources(source) {
        let v = [position[0] - LISTENER[0], position[1] - LISTENER[1], position[2] - LISTENER[2]];
        let distance = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let az = v[1].atan2(v[0]).to_degrees();
        let el = v[2].atan2(v[0].hypot(v[1])).to_degrees();
        let gain = (WALL_REFLECTION.powi(order as i32) * SPEAKER_DISTANCE / distance) as f32;
        let offset = ((distance - SPEAKER_DISTANCE) / SPEED_OF_SOUND * f64::from(RATE)).round() as usize;
        let (mut l, mut r) = hrtf.pair(az, el);
        // Walls absorb more treble than bass: darken each bounce a little.
        for _ in 0..order {
            one_pole_lowpass(&mut l, 7_000.0);
            one_pole_lowpass(&mut r, 7_000.0);
        }
        if order == 0 {
            direct_energy = l.iter().chain(&r).map(|x| f64::from(*x).powi(2)).sum::<f64>();
        }
        add_at(&mut left, &l, offset, gain);
        add_at(&mut right, &r, offset, gain);
    }

    // The late tail: decaying noise, independent in each ear.
    let mut rng = seed;
    let start = (TAIL_START * f64::from(RATE)) as usize;
    let fade_in = (0.012 * f64::from(RATE)) as usize;
    let mut tail = [vec![0f32; LENGTH], vec![0f32; LENGTH]];
    for ear in &mut tail {
        for (i, s) in ear.iter_mut().enumerate().skip(start) {
            let t = (i - start) as f64 / f64::from(RATE);
            let decay = (-6.9078 * t / RT60).exp();
            let ramp = ((i - start) as f64 / fade_in as f64).min(1.0);
            *s = (noise(&mut rng) * decay * ramp) as f32;
        }
        one_pole_lowpass(ear, 6_000.0);
    }
    let tail_energy: f64 = tail.iter().flatten().map(|x| f64::from(*x).powi(2)).sum();
    let scale = if tail_energy > 0.0 { (TAIL_ENERGY * direct_energy / tail_energy).sqrt() as f32 } else { 0.0 };
    add_at(&mut left, &tail[0], 0, scale);
    add_at(&mut right, &tail[1], 0, scale);
    Brir { left, right }
}

/// The source's mirror images in the room up to second order, with how many
/// walls each has bounced off (0 is the source itself).
fn image_sources(source: [f64; 3]) -> Vec<([f64; 3], u32)> {
    // Per axis: (coordinate, bounces) for n in -1..=1 and both parities.
    let axis = |s: f64, size: f64| -> Vec<(f64, u32)> {
        let mut out = Vec::new();
        for n in -1i32..=1 {
            out.push((s + 2.0 * f64::from(n) * size, (2 * n).unsigned_abs()));
            out.push((-s + 2.0 * f64::from(n) * size, (2 * n - 1).unsigned_abs()));
        }
        out
    };
    let (xs, ys, zs) = (axis(source[0], ROOM[0]), axis(source[1], ROOM[1]), axis(source[2], ROOM[2]));
    let mut images = Vec::new();
    for &(x, ox) in &xs {
        for &(y, oy) in &ys {
            for &(z, oz) in &zs {
                let order = ox + oy + oz;
                if order <= 2 {
                    images.push(([x, y, z], order));
                }
            }
        }
    }
    images
}

/// The inverse of the HRTF's average response over the whole sphere, as a
/// minimum-phase FIR of `EQ_TAPS` taps.
fn diffuse_field_inverse(hrtf: &Hrtf) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let bins = FFT_SIZE / 2 + 1;
    let mut power = vec![0f64; bins];
    let mut weight = 0.0;
    for el in [-40.0f64, -20.0, 0.0, 20.0, 40.0, 60.0, 80.0] {
        // Fewer directions near the poles, so each patch of sphere counts once.
        let w = el.to_radians().cos();
        for az in (0..360).step_by(10) {
            let (l, r) = hrtf.pair(f64::from(az), el);
            for ir in [l, r] {
                let mut buf: Vec<Complex32> = ir.iter().map(|&x| Complex32::new(x, 0.0)).collect();
                buf.resize(FFT_SIZE, Complex32::new(0.0, 0.0));
                fft.process(&mut buf);
                for (p, c) in power.iter_mut().zip(&buf) {
                    *p += w * f64::from(c.norm_sqr());
                }
                weight += w;
            }
        }
    }
    let freq = |k: usize| k as f64 * f64::from(RATE) / FFT_SIZE as f64;
    let average: Vec<f64> = power.iter().map(|p| (p / weight).sqrt().max(1e-9)).collect();

    // Smooth to third octaves: the correction should follow the broad shape,
    // not every notch of one dummy head.
    let smoothed: Vec<f64> = (0..bins)
        .map(|k| {
            let f = freq(k).max(1.0);
            let (lo, hi) = (f / 2f64.powf(1.0 / 6.0), f * 2f64.powf(1.0 / 6.0));
            let range = (lo * FFT_SIZE as f64 / f64::from(RATE)) as usize..=((hi * FFT_SIZE as f64 / f64::from(RATE)) as usize).min(bins - 1);
            let n = range.clone().count().max(1) as f64;
            (range.map(|i| average[i].powi(2)).sum::<f64>() / n).sqrt()
        })
        .collect();

    // Level reference: 200-500 Hz, where a head barely changes the sound
    // (above it the ear canal's resonance rises, below it the measurement
    // speaker rolled off), so the correction leaves that band alone.
    let mid: Vec<f64> = (0..bins).filter(|&k| (200.0..500.0).contains(&freq(k))).map(|k| smoothed[k].ln()).collect();
    let reference = (mid.iter().sum::<f64>() / mid.len() as f64).exp();
    let gain_db = |k: usize| {
        let f = freq(k).clamp(EQ_BAND.0, EQ_BAND.1);
        let k = (f * FFT_SIZE as f64 / f64::from(RATE)).round() as usize;
        (20.0 * (reference / smoothed[k]).log10()).clamp(EQ_MAX_CUT, EQ_MAX_BOOST)
    };
    let log_magnitude: Vec<f64> = (0..bins).map(|k| gain_db(k) / 20.0 * std::f64::consts::LN_10).collect();
    minimum_phase(&log_magnitude, &mut planner)
}

/// A minimum-phase FIR with the given log magnitude (natural log, bins 0 to
/// N/2), by the real-cepstrum method, `EQ_TAPS` long with a faded end.
fn minimum_phase(log_magnitude: &[f64], planner: &mut FftPlanner<f32>) -> Vec<f32> {
    let n = FFT_SIZE;
    let forward = planner.plan_fft_forward(n);
    let inverse = planner.plan_fft_inverse(n);
    let mut buf: Vec<Complex32> = (0..n)
        .map(|k| {
            let k = if k <= n / 2 { k } else { n - k };
            Complex32::new(log_magnitude[k] as f32, 0.0)
        })
        .collect();
    inverse.process(&mut buf);
    for c in &mut buf {
        *c /= n as f32;
    }
    // Fold the cepstrum onto positive quefrencies.
    for i in 1..n / 2 {
        buf[i] *= 2.0;
        buf[n - i] = Complex32::new(0.0, 0.0);
    }
    forward.process(&mut buf);
    for c in &mut buf {
        *c = c.exp();
    }
    inverse.process(&mut buf);
    let fade = EQ_TAPS / 8;
    (0..EQ_TAPS)
        .map(|i| {
            let x = buf[i].re / n as f32;
            if i >= EQ_TAPS - fade {
                let t = (i - (EQ_TAPS - fade)) as f32 / fade as f32;
                x * 0.5 * (1.0 + (std::f32::consts::PI * t).cos())
            } else {
                x
            }
        })
        .collect()
}

fn convolve(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let mut out = vec![0f32; signal.len()];
    for (i, &s) in signal.iter().enumerate() {
        if s == 0.0 {
            continue;
        }
        for (j, &f) in filter.iter().enumerate().take(signal.len() - i) {
            out[i + j] += s * f;
        }
    }
    out
}

fn add_at(into: &mut [f32], from: &[f32], offset: usize, gain: f32) {
    for (dst, src) in into.iter_mut().skip(offset).zip(from) {
        *dst += src * gain;
    }
}

fn one_pole_lowpass(x: &mut [f32], cutoff: f64) {
    let a = (-2.0 * std::f64::consts::PI * cutoff / f64::from(RATE)).exp() as f32;
    let mut y = 0f32;
    for s in x {
        y = (1.0 - a) * *s + a * y;
        *s = y;
    }
}

/// Uniform noise in [-1, 1) from xorshift64*, so the tail is the same every
/// time (and needs no dependency).
fn noise(state: &mut u64) -> f64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    let bits = state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11;
    bits as f64 / (1u64 << 52) as f64 - 1.0
}

/// Writes the BRIRs as one float WAV, left then right ear for each speaker.
pub fn write_wav(path: &Path, brirs: &[Brir]) -> Result<(), String> {
    let channels: Vec<&Vec<f32>> = brirs.iter().flat_map(|b| [&b.left, &b.right]).collect();
    let frames = channels.first().map_or(0, |c| c.len());
    let (n, bytes_per) = (channels.len() as u16, 4u16);
    let data_len = (frames * channels.len() * 4) as u32;
    let mut out = Vec::with_capacity(data_len as usize + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * u32::from(n * bytes_per)).to_le_bytes());
    out.extend_from_slice(&(n * bytes_per).to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        for c in &channels {
            out.extend_from_slice(&c[i].to_le_bytes());
        }
    }
    let tmp = path.with_extension("wav.tmp");
    std::fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(&out))
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEMAR: &str = "/usr/share/libmysofa/MIT_KEMAR_normal_pinna.sofa";

    fn energy(x: &[f32]) -> f64 {
        x.iter().map(|v| f64::from(*v).powi(2)).sum()
    }

    fn onset(x: &[f32]) -> usize {
        let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        x.iter().position(|v| v.abs() > peak * 0.2).unwrap_or(0)
    }

    #[test]
    fn image_sources_count_the_source_and_its_first_two_orders() {
        let images = image_sources([1.0, 1.0, 1.0]);
        assert_eq!(images.iter().filter(|(_, o)| *o == 0).count(), 1);
        assert_eq!(images.iter().filter(|(_, o)| *o == 1).count(), 6);
        assert_eq!(images.iter().filter(|(_, o)| *o == 2).count(), 18);
    }

    #[test]
    fn minimum_phase_of_flat_is_an_impulse() {
        let ir = minimum_phase(&vec![0.0; FFT_SIZE / 2 + 1], &mut FftPlanner::new());
        assert!((ir[0] - 1.0).abs() < 1e-3);
        assert!(ir[1..].iter().all(|v| v.abs() < 1e-3));
    }

    /// The direction convention, checked on the real data: a source on the
    /// left is louder, and arrives first, in the left ear.
    #[test]
    fn left_speakers_are_on_the_left() {
        if !Path::new(KEMAR).exists() {
            return;
        }
        let brirs = build(Path::new(KEMAR), &[30.0, 330.0, 90.0, 270.0]).unwrap();
        for (b, left_side) in brirs.iter().zip([true, false, true, false]) {
            let (l, r) = (energy(&b.left[..2400]), energy(&b.right[..2400]));
            assert_eq!(l > r, left_side, "left {l} right {r}");
            assert_eq!(onset(&b.left) < onset(&b.right), left_side);
        }
        assert!(brirs.iter().all(|b| b.left.len() == LENGTH && b.right.len() == LENGTH));
        assert!(brirs.iter().flat_map(|b| b.left.iter().chain(&b.right)).all(|v| v.is_finite()));
    }
}

#[cfg(test)]
mod report {
    use super::*;

    /// `cargo test -p quantumengined brir::report -- --ignored --nocapture`
    #[test]
    #[ignore = "prints numbers for a human to read"]
    fn print_correction_and_room_levels() {
        let kemar = Path::new("/usr/share/libmysofa/MIT_KEMAR_normal_pinna.sofa");
        let hrtf = Hrtf::open(kemar).unwrap();
        let eq = diffuse_field_inverse(&hrtf);
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let mut buf: Vec<Complex32> = eq.iter().map(|&x| Complex32::new(x, 0.0)).collect();
        buf.resize(FFT_SIZE, Complex32::new(0.0, 0.0));
        fft.process(&mut buf);
        for f in [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 12000.0, 16000.0] {
            let k = (f * FFT_SIZE as f64 / f64::from(RATE)).round() as usize;
            println!("correction {f:>7} Hz: {:+.1} dB", 20.0 * f64::from(buf[k].norm()).log10());
        }
        for (az, b) in [0.0, 30.0, 90.0, 150.0].iter().zip(build(kemar, &[0.0, 30.0, 90.0, 150.0]).unwrap()) {
            let split = (0.0025 * f64::from(RATE)) as usize + 64;
            let e = |x: &[f32]| x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
            let direct = e(&b.left[..split]) + e(&b.right[..split]);
            let rest = e(&b.left[split..]) + e(&b.right[split..]);
            let peak = b.left.iter().chain(&b.right).fold(0f32, |m, v| m.max(v.abs()));
            println!("az {az:>5}: direct/room {:+.1} dB, peak {peak:.3}", 10.0 * (direct / rest).log10());
        }
    }
}
