use num_enum::{FromPrimitive};
use core::f32;
use micromath::F32Ext;

use core::fmt::{Debug, Formatter};
use embassy_time::Instant;

#[derive(Debug, FromPrimitive, Copy, Clone)]
#[repr(u8)]
pub enum Waveform {
    #[num_enum(default)]
    Triangle,
    Sine,
    Saw,
    RevSaw,
    Square,
    // Random,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

impl Debug for Lfo {
    fn fmt(&self, _f: &mut Formatter<'_>) -> core::fmt::Result {
        // TODO
        Ok(())
    }
}

pub struct Lfo {
    offset: Instant,
    period: f32,
    // between 0 and 1
    amount: f32,
    wave: Waveform,
}

impl Default for Lfo {
    fn default() -> Self {
        Self {
            offset: Instant::now(),
            period: 200.0,
            amount: 0.0,
            wave: Default::default(),
        }
    }
}

// Yes, these computations are HORRIBLY INEFFICIENT and naive. IJDGAF.
impl Lfo {
    pub fn mod_value(&mut self, froot: f32/*, chaos: &mut WyRand*/) -> f32 {
        let now = Instant::now();
        let time = (now - self.offset).as_millis() as f32;
        (froot + match self.wave {
            Waveform::Triangle => {
                let timex = time % self.period;
                let half = self.period / 2.0;
                let mut modulation = timex / half;
                if timex > half {
                    modulation = 1.0 - modulation;
                }
                (modulation - 0.5) * 2.0 * self.amount
            }
            Waveform::Sine => {
                let timex = time / self.period;
                timex.sin() * self.amount
            }
            Waveform::Square => {
                let timex = time % self.period;
                let half = self.period / 2.0;
                (if timex > half { 1.0 } else { -1.0 }) * self.amount
            }
            Waveform::Saw => {
                let timex = time / self.period;
                ((1.0 - timex.fract()) - 0.5) * 2.0 * self.amount
            }
            Waveform::RevSaw => {
                let timex = time / self.period;
                (timex.fract() - 0.5) * 2.0 * self.amount
            }
            // Waveform::Random => ((chaos.generate_range::<u32>(0, u32::MAX) as f32 / u32::MAX as f32) - 0.5) * 2.0 * self.amount
        }).max(0.0).min(1.0)
    }

    pub fn get_amount(&self) -> f32 {
        self.amount
    }

    pub fn set_amount(&mut self, mut amount: f32) {
        amount = amount.max(0.0).min(1.0);
        self.amount = amount
    }

    pub fn get_rate_hz(&self) -> f32 {
        1000.0 / self.period
    }

    pub fn set_rate_hz(&mut self, rate: f32) {
        self.period = 1000.0 / rate;
    }

    pub fn get_waveform(&self) -> Waveform {
        self.wave
    }

    pub fn set_waveform(&mut self, wave: Waveform) {
        self.wave = wave;
    }
}