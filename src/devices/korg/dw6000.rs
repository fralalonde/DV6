//! From https://www.untergeek.de/2014/11/taming-arturias-beatstep-sysex-codes-for-programming-via-ipad/
//! Thanks to Richard Wanderlöf and Untergeek
//! Switching the LEDs on and off:
#![allow(dead_code)]

use crate::sysex::{SysexMatcher, Token, Tag, SysexSeq};
use Token::{Seq, Cap, Val, Buf};
use Tag::*;
use alloc::vec::Vec;

const KORG: u8 = 0x42;
const DW_6000_ID: u8 = 0x04;

const ID_FORMAT: u8 = 0x40;
const DATA_FORMAT: u8 = 0x30;

const WRITE_OK: u8 = 0x21;
const WRITE_ERR: u8 = 0x22;

const ID_HEADER: &[u8] = &[KORG, ID_FORMAT];
const DATA_HEADER: &[u8] = &[KORG, DATA_FORMAT, DW_6000_ID];

pub fn id_request_sysex() -> SysexSeq {
    SysexSeq::new(vec![Seq(ID_HEADER)])
}

pub fn id_matcher() -> SysexMatcher {
    SysexMatcher::new(vec![Seq(ID_HEADER), Val(DW_6000_ID)])
}

pub fn write_program_sysex(program: u8) -> SysexSeq {
    SysexSeq::new(vec![Seq(DATA_HEADER), Val(0x11), Val(program)])
}

pub fn load_program_sysex(dump: Vec<u8>) -> SysexSeq {
    SysexSeq::new(vec![Seq(DATA_HEADER), Buf(dump)])
}

pub fn set_parameter_sysex(param: u8, value: u8) -> SysexSeq {
    SysexSeq::new(vec![Seq(DATA_HEADER), Val(0x41), Val(param), Val(value)])
}

pub fn write_matcher() -> SysexMatcher {
    SysexMatcher::new(vec![Seq(DATA_HEADER), Cap(ValueU7)])
}

pub fn dump_request_sysex() -> SysexSeq {
    SysexSeq::new(vec![Seq(DATA_HEADER), Val(0x10)])
}

pub fn dump_matcher() -> SysexMatcher {
    SysexMatcher::new(vec![Seq(DATA_HEADER), Val(0x40), Cap(Dump(26))])
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Dw6Param {
    Osc1Wave,
    Osc1Level,
    Osc1Octave,
    Osc2Wave,
    Osc2Level,
    Osc2Octave,
    Osc2Detune,
    Interval,
    Noise,
    Cutoff,
    Resonance,
    VcfInt,
    VcfAttack,
    VcfDecay,
    VcfBreak,
    VcfSlope,
    VcfSustain,
    VcfRelease,
    VcaAttack,
    VcaDecay,
    VcaBreak,
    VcaSlope,
    VcaSustain,
    VcaRelease,
    BendVcf,
    BendOsc,
    AssignMode,
    Portamento,
    MgFreq,
    MgDelay,
    MgOsc,
    MgVcf,
    KbdTrack,
    Polarity,
    Chorus,
}

#[repr(C)]
#[derive(Debug)]
pub struct Dw6Dump {
    pub assign_mode_bend_osc: AssignModeBendOsc,
    pub portamento_time: Portamento,
    pub osc1_level: Osc1Level,
    pub osc2_level: Osc2Level,
    pub noise_level: Noise,

    pub cutoff: Cutoff,
    pub resonance: Resonance,

    pub vcf_eg_int: VcfInt,
    pub vcf_eg_attack: VcfAttack,
    pub vcf_eg_decay: VcfDecay,
    pub vcf_eg_breakpoint: VcfBreak,
    pub vcf_eg_slope: VcfSlope,
    pub vcf_eg_sustain: VcfSustain,
    pub vcf_eg_release: VcfRelease,

    pub vca_eg_attack: VcaAttack,
    pub vca_eg_decay: VcaDecay,
    pub vca_eg_breakpoint: VcaBreak,
    pub vca_eg_slope: VcaSlope,
    pub bend_vcf_vca_eg_sustain: BendVcfVcaSustain,
    pub osc1_oct_vca_eg_release: Osc1OctVcaRelease,

    pub osc2_oct_mg_freq: Osc2OctMgFreq,
    pub kbd_track_mg_delay: KbdTrackMgDelay,
    pub polarity_mg_osc: PolarityMgOsc,
    pub chorus_mg_vcf: ChrorusMgVcf,

    pub osc1_wave_osc2_wave: Osc1WaveOsc2Wave,
    pub osc2_interval_osc2_detune: IntervalOsc2Detune,
}

pub fn as_dump_ref_mut(buf: &[u8]) -> &mut Dw6Dump {
    let p: *mut Dw6Dump = buf.as_ptr() as *mut Dw6Dump;
    unsafe { &mut *p }
}

pub fn as_dump_ref(buf: &[u8]) -> &Dw6Dump {
    let p: *const Dw6Dump = buf.as_ptr() as *const Dw6Dump;
    unsafe { &*p }
}

pub fn get_param_value(param: Dw6Param, dump_buf: &[u8]) -> u8 {
    use Dw6Param::*;
    let dump = as_dump_ref(dump_buf);
    match param {
        Osc1Wave => dump.osc1_wave_osc2_wave.osc1_waveform(),
        Osc1Level => dump.osc1_level.osc1_level(),
        Osc1Octave => dump.osc1_oct_vca_eg_release.osc1_octave(),
        Osc2Wave => dump.osc1_wave_osc2_wave.osc2_waveform(),
        Osc2Level => dump.osc2_level.osc2_level(),
        Osc2Octave => dump.osc2_oct_mg_freq.osc2_octave(),
        Osc2Detune => dump.osc2_interval_osc2_detune.osc2_detune(),
        Interval => dump.osc2_interval_osc2_detune.osc2_interval(),
        Noise => dump.noise_level.noise_level(),
        Cutoff => dump.cutoff.cutoff(),
        Resonance => dump.resonance.resonance(),
        VcfInt => dump.vcf_eg_int.vcf_eg_int(),
        VcfAttack => dump.vcf_eg_attack.vcf_eg_attack(),
        VcfDecay => dump.vcf_eg_decay.vcf_eg_decay(),
        VcfBreak => dump.vcf_eg_breakpoint.vcf_eg_breakpoint(),
        VcfSlope => dump.vcf_eg_slope.vcf_eg_slope(),
        VcfSustain => dump.vcf_eg_sustain.vcf_eg_sustain(),
        VcfRelease => dump.vcf_eg_release.vcf_eg_release(),
        VcaAttack => dump.vca_eg_attack.vca_eg_attack(),
        VcaDecay => dump.vca_eg_decay.vca_eg_decay(),
        VcaBreak => dump.vca_eg_breakpoint.vca_eg_breakpoint(),
        VcaSlope => dump.vca_eg_slope.vca_eg_slope(),
        VcaSustain => dump.bend_vcf_vca_eg_sustain.vca_eg_sustain(),
        VcaRelease => dump.osc1_oct_vca_eg_release.vca_eg_release(),
        BendVcf => dump.bend_vcf_vca_eg_sustain.bend_vcf(),
        BendOsc => dump.assign_mode_bend_osc.bend_osc(),
        AssignMode => dump.assign_mode_bend_osc.assign_mode(),
        Portamento => dump.portamento_time.portamento_time(),
        MgFreq => dump.osc2_oct_mg_freq.mg_freq(),
        MgDelay => dump.kbd_track_mg_delay.mg_delay(),
        MgOsc => dump.polarity_mg_osc.mg_osc(),
        MgVcf => dump.chorus_mg_vcf.mg_vcf(),
        KbdTrack => dump.kbd_track_mg_delay.kbd_track(),
        Polarity => dump.polarity_mg_osc.polarity(),
        Chorus => dump.chorus_mg_vcf.chorus(),
    }
}

pub fn set_param_value(param: Dw6Param, value: u8, dump_buf: &[u8]) {
    use Dw6Param::*;
    let dump = as_dump_ref_mut(dump_buf);
    match param {
        Osc1Wave => dump.osc1_wave_osc2_wave.set_osc1_waveform(value),
        Osc1Level => dump.osc1_level.set_osc1_level(value),
        Osc1Octave => dump.osc1_oct_vca_eg_release.set_osc1_octave(value),
        Osc2Wave => dump.osc1_wave_osc2_wave.set_osc2_waveform(value),
        Osc2Level => dump.osc2_level.set_osc2_level(value),
        Osc2Octave => dump.osc2_oct_mg_freq.set_osc2_octave(value),
        Osc2Detune => dump.osc2_interval_osc2_detune.set_osc2_detune(value),
        Interval => dump.osc2_interval_osc2_detune.set_osc2_interval(value),
        Noise => dump.noise_level.set_noise_level(value),
        Cutoff => dump.cutoff.set_cutoff(value),
        Resonance => dump.resonance.set_resonance(value),
        VcfInt => dump.vcf_eg_int.set_vcf_eg_int(value),
        VcfAttack => dump.vcf_eg_attack.set_vcf_eg_attack(value),
        VcfDecay => dump.vcf_eg_decay.set_vcf_eg_decay(value),
        VcfBreak => dump.vcf_eg_breakpoint.set_vcf_eg_breakpoint(value),
        VcfSlope => dump.vcf_eg_slope.set_vcf_eg_slope(value),
        VcfSustain => dump.vcf_eg_sustain.set_vcf_eg_sustain(value),
        VcfRelease => dump.vcf_eg_release.set_vcf_eg_release(value),
        VcaAttack => dump.vca_eg_attack.set_vca_eg_attack(value),
        VcaDecay => dump.vca_eg_decay.set_vca_eg_decay(value),
        VcaBreak => dump.vca_eg_breakpoint.set_vca_eg_breakpoint(value),
        VcaSlope => dump.vca_eg_slope.set_vca_eg_slope(value),
        VcaSustain => dump.bend_vcf_vca_eg_sustain.set_vca_eg_sustain(value),
        VcaRelease => dump.osc1_oct_vca_eg_release.set_vca_eg_release(value),
        BendVcf => dump.bend_vcf_vca_eg_sustain.set_bend_vcf(value),
        BendOsc => dump.assign_mode_bend_osc.set_bend_osc(value),
        AssignMode => dump.assign_mode_bend_osc.set_assign_mode(value),
        Portamento => dump.portamento_time.set_portamento_time(value),
        MgFreq => dump.osc2_oct_mg_freq.set_mg_freq(value),
        MgDelay => dump.kbd_track_mg_delay.set_mg_delay(value),
        MgOsc => dump.polarity_mg_osc.set_mg_osc(value),
        MgVcf => dump.chorus_mg_vcf.set_mg_vcf(value),
        KbdTrack => dump.kbd_track_mg_delay.set_kbd_track(value),
        Polarity => dump.polarity_mg_osc.set_polarity(value),
        Chorus => dump.chorus_mg_vcf.set_chrorus(value),
    }
}

impl Dw6Param {
    pub fn max_value(&self) -> u8 {
        use Dw6Param::*;
        match self {
            Osc2Detune | Interval |
            Osc1Wave | Osc2Wave => 7,

            AssignMode | KbdTrack |
            Osc1Octave | Osc2Octave => 3,

            Cutoff => 63,

            Resonance |
            Portamento |
            Osc2Level | Osc1Level | Noise |
            MgFreq | MgDelay | MgOsc | MgVcf |
            VcfInt | VcfAttack | VcfDecay | VcfBreak | VcfSlope | VcfSustain | VcfRelease |
            VcaAttack | VcaDecay | VcaBreak | VcaSlope | VcaSustain | VcaRelease => 31,

            Polarity | Chorus | BendVcf => 1,

            BendOsc => 15,
        }
    }


    pub fn dump_index(&self) -> usize {
        use Dw6Param::*;
        match self {
            AssignMode | BendOsc => 0,
            Portamento => 1,
            Osc1Level => 2,
            Osc2Level => 3,
            Noise => 4,
            Cutoff => 5,
            Resonance => 6,
            VcfInt => 7,
            VcfAttack => 8,
            VcfDecay => 9,
            VcfBreak => 10,
            VcfSlope => 11,
            VcfSustain => 12,
            VcfRelease => 13,
            VcaAttack => 14,
            VcaDecay => 15,
            VcaBreak => 16,
            VcaSlope => 17,
            BendVcf | VcaSustain => 18,
            Osc1Octave | VcaRelease => 19,
            Osc2Octave | MgFreq => 20,
            KbdTrack | MgDelay => 21,
            Polarity | MgOsc => 22,
            Chorus | MgVcf => 23,
            Osc1Wave | Osc2Wave => 24,
            Osc2Detune | Interval => 25,
        }
    }

    pub fn dump_value(&self, dump_buf: &[u8]) -> u8 {
        use Dw6Param::*;
        let dump = as_dump_ref(dump_buf);
        match self {
            AssignMode | BendOsc => dump.assign_mode_bend_osc.0,
            Portamento => dump.portamento_time.0,
            Osc1Level => dump.osc1_level.0,
            Osc2Level => dump.osc2_level.0,
            Noise => dump.noise_level.0,
            Cutoff => dump.cutoff.0,
            Resonance => dump.resonance.0,
            VcfInt => dump.vcf_eg_int.0,
            VcfAttack => dump.vcf_eg_attack.0,
            VcfDecay => dump.vcf_eg_decay.0,
            VcfBreak => dump.vcf_eg_breakpoint.0,
            VcfSlope => dump.vcf_eg_slope.0,
            VcfSustain => dump.vcf_eg_sustain.0,
            VcfRelease => dump.vcf_eg_release.0,
            VcaAttack => dump.vca_eg_attack.0,
            VcaDecay => dump.vca_eg_decay.0,
            VcaBreak => dump.vca_eg_breakpoint.0,
            VcaSlope => dump.vca_eg_slope.0,
            BendVcf | VcaSustain => dump.bend_vcf_vca_eg_sustain.0,
            Osc1Octave | VcaRelease => dump.osc1_oct_vca_eg_release.0,
            Osc2Octave | MgFreq => dump.osc2_oct_mg_freq.0,
            KbdTrack | MgDelay => dump.kbd_track_mg_delay.0,
            Polarity | MgOsc => dump.polarity_mg_osc.0,
            Chorus | MgVcf => dump.chorus_mg_vcf.0,
            Osc1Wave | Osc2Wave => dump.osc1_wave_osc2_wave.0,
            Osc2Detune | Interval => dump.osc2_interval_osc2_detune.0,
        }
    }
}

bitfield! {
    pub struct AssignModeBendOsc(u8);
    impl Debug;
    pub assign_mode, set_assign_mode: 5, 4;
    pub bend_osc, set_bend_osc: 3, 0;
}

bitfield! {
    pub struct Portamento(u8); impl Debug;
    pub portamento_time, set_portamento_time: 4, 0;
}

bitfield! {
    pub struct Osc1Level(u8); impl Debug;
    pub osc1_level, set_osc1_level: 4, 0;
}

bitfield! {
    pub struct Osc2Level(u8); impl Debug;
    pub osc2_level, set_osc2_level: 4, 0;
}

bitfield! {
    pub struct Noise(u8); impl Debug;
    pub noise_level, set_noise_level: 4, 0;
}

bitfield! {
    pub struct Cutoff(u8); impl Debug;
    pub cutoff, set_cutoff: 5, 0;
}

bitfield! {
    pub struct Resonance(u8); impl Debug;
    pub resonance, set_resonance: 4, 0;
}

bitfield! {
    pub struct VcfInt(u8); impl Debug;
    pub vcf_eg_int, set_vcf_eg_int: 4,0;
}

bitfield! {
    pub struct VcfAttack(u8); impl Debug;
    pub vcf_eg_attack, set_vcf_eg_attack: 4,0;
}

bitfield! {
    pub struct VcfDecay(u8); impl Debug;
    pub vcf_eg_decay, set_vcf_eg_decay: 4,0;
}

bitfield! {
    pub struct VcfBreak(u8); impl Debug;
    pub vcf_eg_breakpoint, set_vcf_eg_breakpoint: 4,0;
}

bitfield! {
    pub struct VcfSlope(u8); impl Debug;
    pub vcf_eg_slope, set_vcf_eg_slope: 4,0;
}

bitfield! {
    pub struct VcfSustain(u8); impl Debug;
    pub vcf_eg_sustain, set_vcf_eg_sustain: 4,0;
}

bitfield! {
    pub struct VcfRelease(u8); impl Debug;
    pub vcf_eg_release, set_vcf_eg_release: 4,0;
}

bitfield! {
    pub struct VcaAttack(u8); impl Debug;
    pub vca_eg_attack, set_vca_eg_attack: 4,0;
}

bitfield! {
    pub struct VcaDecay(u8); impl Debug;
    pub vca_eg_decay, set_vca_eg_decay: 4,0;
}

bitfield! {
    pub struct VcaBreak(u8); impl Debug;
    pub vca_eg_breakpoint, set_vca_eg_breakpoint: 4,0;
}

bitfield! {
    pub struct VcaSlope(u8); impl Debug;
    pub vca_eg_slope, set_vca_eg_slope: 4,0;
}

bitfield! {
    pub struct BendVcfVcaSustain(u8); impl Debug;
    pub bend_vcf, set_bend_vcf: 5,5;
    pub vca_eg_sustain, set_vca_eg_sustain: 4,0;
}

bitfield! {
    pub struct Osc1OctVcaRelease(u8); impl Debug;
    pub osc1_octave, set_osc1_octave: 6,5;
    pub vca_eg_release, set_vca_eg_release: 4,0;
}

bitfield! {
    pub struct Osc2OctMgFreq(u8); impl Debug;
    pub osc2_octave, set_osc2_octave: 6,5;
    pub mg_freq, set_mg_freq: 4,0;
}

bitfield! {
    pub struct KbdTrackMgDelay(u8); impl Debug;
    pub kbd_track, set_kbd_track: 6,5;
    pub mg_delay, set_mg_delay: 4,0;
}

bitfield! {
    pub struct PolarityMgOsc(u8); impl Debug;
    pub polarity, set_polarity: 5,5;
    pub mg_osc, set_mg_osc: 4,0;
}

bitfield! {
    pub struct ChrorusMgVcf(u8); impl Debug;
    pub chorus, set_chrorus: 5,5;
    pub mg_vcf, set_mg_vcf: 4,0;
}

bitfield! {
    pub struct Osc1WaveOsc2Wave(u8); impl Debug;
    pub osc1_waveform, set_osc1_waveform: 5,3;
    pub osc2_waveform, set_osc2_waveform: 2,0;
}

bitfield! {
    pub struct IntervalOsc2Detune(u8); impl Debug;
    pub osc2_interval, set_osc2_interval: 5,3;
    pub osc2_detune, set_osc2_detune: 2,0;
}
