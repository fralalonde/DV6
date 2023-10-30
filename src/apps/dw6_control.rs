//! Sends MIDI to Korg DW-6000 acccording to messages
//!
use midi::{MidiMessage, Note, program_change, MidiError, U7, PacketList, channel};

use crate::{midi, MIDI_DIN_1_IN, MIDI_DIN_2_IN, MIDI_DIN_2_OUT, sysex};

use core::convert::TryFrom;

use core::slice;

use embassy_executor::{Spawner, SpawnError};
use embassy_time::{Instant, Timer, Duration};

use num_enum::TryFromPrimitive;
use num::{Integer};
use crate::apps::lfo::{Lfo, Waveform};

use crate::devices::korg::dw6000;

use hashbrown::HashMap;
use heapless::Vec;

use midi::{capture_sysex, SysexCapture};
use crate::devices::korg::dw6000::Dw6Param;
use crate::resource::{Shared};

const SHORT_PRESS_MS: Duration = Duration::from_millis(250);

static DW6_CTRL: Shared<Dw6ControlInner> = Shared::uninit("DW6_CTRL");

static DW6_SYSEX_DUMP: Shared<Vec<u8, 26>> = Shared::uninit("DW6_SYSEX_DUMP");

#[embassy_executor::task]
async fn bstep_rx() -> ! {
    let mut bstep_in = MIDI_DIN_1_IN.lock().await;
    loop {
        if let Ok(packet) = bstep_in.get_mut().unwrap().receive().await {
            packets_from_beatstep(PacketList::single(packet)).await;
        }
    }
}

#[embassy_executor::task]
async fn dw6_rx() -> ! {
    // exclusive locked FOREVER muahahaha
    let mut dw6_in = MIDI_DIN_2_IN.lock().await;
    loop {
        match dw6_in.get_mut().unwrap().receive().await {
            Ok(packet) => packets_from_dw_6000(PacketList::single(packet)).await,
            Err(midi_err) => error!("dw6 rx error {}", midi_err),
        }
    }
}

#[embassy_executor::task]
async fn dw6_dump_request() -> ! {
    loop {
        let _ = dw6_send(PacketList::from_iter(dw6000::dump_request_sysex())).await;
        Timer::after(Duration::from_millis(250)).await;
    }
}

#[embassy_executor::task]
async fn lfo_mod() -> ! {
    loop {
        // LFO2 modulation
        let mut state = DW6_CTRL.lock().await;
        let state = state.get_mut().unwrap();
        if let Some(lfo2_param) = state.lfo2_param.map(dw6000::Dw6Param::from) {
            if let Some(root) = state.mod_dump.get(&lfo2_param).cloned() {
                let max = lfo2_param.max_value();
                let fmax = max as f32;
                let froot: f32 = root as f32 / fmax;

                let fmod = state.lfo2.mod_value(froot /*chaos*/) * fmax;
                let mod_value = fmod.max(0.0).min(fmax) as u8;

                if let Some(dump) = &state.current_dump {
                    dw6000::set_param_value(lfo2_param, mod_value, dump.as_slice());
                    let sysex = param_set_sysex(lfo2_param, dump);
                    let _ = dw6_send(PacketList::from_iter(sysex)).await;
                }
            }
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AppError {
    Init,
    Spawn(SpawnError)
}

impl From<SpawnError> for AppError {
    fn from(value: SpawnError) -> Self {
        AppError::Spawn(value)
    }
}

pub async fn start_app(spawner: Spawner) -> Result<(), AppError> {
    DW6_CTRL.lock().await.set(Dw6ControlInner {
        current_dump: None,
        mod_dump: HashMap::new(),
        base_page: KnobPage::Osc,
        temp_page: None,
        bank: None,
        lfo2: Lfo::default(),
        lfo2_param: None,
    }).map_err(|_| AppError::Init)?;

    DW6_SYSEX_DUMP.lock().await.set(Vec::new()).map_err(|_| AppError::Init)?;

    spawner.spawn(bstep_rx())?;

    unwrap!(spawner.spawn(dw6_rx()));

    // unwrap!(spawner.spawn(lfo_mod()));

    spawner.spawn(dw6_dump_request())?;

    info!("DW6000 Controller Active");
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
enum KnobPage {
    Osc = 0,
    Env = 1,
    Mod = 2,
    Arp = 3,
}


#[derive(Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
enum TogglePage {
    Arp = 4,
    Latch = 5,
    Polarity = 6,
    Chorus = 7,
}

#[derive(Debug, Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum Lfo2Dest {
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
    MgFreq,
    MgDelay,
    MgOsc,
    MgVcf,
}

impl From<Lfo2Dest> for dw6000::Dw6Param {
    fn from(dest: Lfo2Dest) -> Self {
        match dest {
            Lfo2Dest::Osc1Wave => Dw6Param::Osc1Wave,
            Lfo2Dest::Osc1Level => Dw6Param::Osc1Level,
            Lfo2Dest::Osc1Octave => Dw6Param::Osc1Octave,
            Lfo2Dest::Osc2Wave => Dw6Param::Osc2Wave,
            Lfo2Dest::Osc2Level => Dw6Param::Osc2Level,
            Lfo2Dest::Osc2Octave => Dw6Param::Osc2Octave,
            Lfo2Dest::Osc2Detune => Dw6Param::Osc2Detune,
            Lfo2Dest::Interval => Dw6Param::Interval,
            Lfo2Dest::Noise => Dw6Param::Noise,
            Lfo2Dest::Cutoff => Dw6Param::Cutoff,
            Lfo2Dest::Resonance => Dw6Param::Resonance,
            Lfo2Dest::VcfInt => Dw6Param::VcfInt,
            Lfo2Dest::VcfAttack => Dw6Param::VcfAttack,
            Lfo2Dest::VcfDecay => Dw6Param::VcfDecay,
            Lfo2Dest::VcfBreak => Dw6Param::VcfBreak,
            Lfo2Dest::VcfSlope => Dw6Param::VcfSlope,
            Lfo2Dest::VcfSustain => Dw6Param::VcfSustain,
            Lfo2Dest::VcfRelease => Dw6Param::VcfRelease,
            Lfo2Dest::VcaAttack => Dw6Param::VcaAttack,
            Lfo2Dest::VcaDecay => Dw6Param::VcaDecay,
            Lfo2Dest::VcaBreak => Dw6Param::VcaBreak,
            Lfo2Dest::VcaSlope => Dw6Param::VcaSlope,
            Lfo2Dest::VcaSustain => Dw6Param::VcaSustain,
            Lfo2Dest::VcaRelease => Dw6Param::VcaRelease,
            Lfo2Dest::MgFreq => Dw6Param::MgFreq,
            Lfo2Dest::MgDelay => Dw6Param::MgDelay,
            Lfo2Dest::MgOsc => Dw6Param::MgOsc,
            Lfo2Dest::MgVcf => Dw6Param::MgVcf,
        }
    }
}

/// DW600 patch dump is 26 bytes sysex
const DUMP_LENGTH: usize = 26;

#[derive(Debug)]
struct Dw6ControlInner {
    current_dump: Option<Vec<u8, DUMP_LENGTH>>,
    // saved values from dump before being modulated
    mod_dump: HashMap<dw6000::Dw6Param, u8>,
    base_page: KnobPage,
    // if temp_page is released quickly, is becomes base_page
    temp_page: Option<(KnobPage, Instant)>,
    bank: Option<u8>,
    lfo2: Lfo,
    lfo2_param: Option<Lfo2Dest>,
    // arp_enabled: bool,
    // arp_mode: ArpMode,
    // arp_oct: u8, // 1..4
}

impl Dw6ControlInner {
    fn active_page(&self) -> KnobPage {
        self.temp_page.map(|p| p.0).unwrap_or(self.base_page)
    }
}

fn note_page(note: Note) -> Option<KnobPage> {
    KnobPage::try_from(note as u8).ok()
}

fn toggle_page(note: Note) -> Option<TogglePage> {
    TogglePage::try_from(note as u8).ok()
}

fn note_bank(note: Note) -> Option<u8> {
    let note_u8 = note as u8;
    match note_u8.div_rem(&8) {
        (1, n) => Some(n),
        _ => None,
    }
}

fn note_prog(note: Note) -> Option<u8> {
    let note_u8 = note as u8;
    match note_u8.div_rem(&8) {
        (0, n) => Some(n),
        _ => None,
    }
}


impl Dw6ControlInner {
    fn set_modulated(&mut self, p: dw6000::Dw6Param, root_value: u8) {
        self.mod_dump.insert(p, root_value);
    }

    async fn unset_modulated(&mut self, p: dw6000::Dw6Param) -> Result<(), MidiError> {
        if let Some(root) = self.mod_dump.remove(&p) {
            if let Some(dump) = &mut self.current_dump {
                let z = unsafe { slice::from_raw_parts_mut(dump.as_mut_ptr(), dump.len()) };
                dw6000::set_param_value(p, root, z);
                self.send_param_value(p).await?
            }
        }
        Ok(())
    }

    async fn send_param_value(&mut self, param: dw6000::Dw6Param) -> Result<(), MidiError> {
        if let Some(dump) = &self.current_dump {
            dw6_send(param_set_sysex(param, dump)).await?
        }
        Ok(())
    }
}


async fn toggle_param(param: dw6000::Dw6Param, dump: &mut Vec<u8, DUMP_LENGTH>) -> Result<(), MidiError> {
    let mut value = dw6000::get_param_value(param, dump);
    value ^= 1;
    dw6000::set_param_value(param, value, &dump);
    dw6_send(param_set_sysex(param, dump)).await
}

async fn packets_from_beatstep(packets: PacketList) {
    for packet in packets.0.into_iter() {
        if let Ok(msg) = MidiMessage::try_from(packet) {
            if let Err(err) = msg_from_beatstep(msg).await {
                error!("{}", err);
            }
        }
    }
}

async fn dw6_send(packets: impl Into<PacketList>) -> Result<(), MidiError> {
    let mut dw6_out = MIDI_DIN_2_OUT.lock().await;
    dw6_out.get_mut().unwrap().transmit(packets.into()).await
}

async fn msg_from_beatstep(msg: MidiMessage) -> Result<bool, MidiError> {
    let mut state = DW6_CTRL.lock().await;
    let state = state.get_mut().unwrap();
    match msg {
        MidiMessage::NoteOn(_, note, _) => {
            if let Some(bank) = note_bank(note) {
                state.bank = Some(bank)
            } else if let Some(prog) = note_prog(note) {
                if let Some(bank) = state.bank {
                    // TODO parameterize channel
                    let pc = program_change(channel(1)?, (bank * 8) + prog)?;
                    dw6_send(PacketList::single(pc.into())).await?;
                }
            }
            if let Some(page) = note_page(note) {
                state.temp_page = Some((page, Instant::now()));
            }
            if let Some(tog) = toggle_page(note) {
                if let Some(dump) = &mut state.current_dump {
                    match tog {
                        TogglePage::Arp => {}
                        TogglePage::Latch => {}
                        TogglePage::Polarity => toggle_param(Dw6Param::Polarity, dump).await?,
                        TogglePage::Chorus => toggle_param(Dw6Param::Chorus, dump).await?,
                    }
                }
            }
        }
        MidiMessage::NoteOff(_, note, _) => {
            if state.bank == note_bank(note) {
                state.bank = None
            }
            if let Some((temp_page, press_start_ms)) = state.temp_page {
                if let Some(note_page) = note_page(note) {
                    if note_page == temp_page {
                        let held_for_ms = Instant::now() - press_start_ms;
                        if held_for_ms < SHORT_PRESS_MS {
                            state.base_page = temp_page;
                        }
                        state.temp_page = None;
                    }
                }
            }
        }
        MidiMessage::ControlChange(_ch, cc, value) =>
            if let Some(param) = cc_to_dw_param(cc, state.active_page()) {
                if let Some(root) = state.mod_dump.get_mut(&param) {
                    *root = value.0
                } else if let Some(dump) = &mut state.current_dump {
                    dw6000::set_param_value(param, value.into(), &dump);
                    dw6_send(param_set_sysex(param, dump)).await?

                    // context.packets.clear();
                    // context.packets.extend(param_to_sysex(param, dump));
                    // context.strings.push(format!("{:?}\n{:?}", param, get_param_value(param, dump)));
                } else {
                    info!("no dump yet");
                }
            } else if let Some(param) = cc_to_ctl_param(cc, state.active_page()) {
                match param {
                    CtlParam::Lfo2Rate => {
                        let base_rate = (value.0 as f32 + 1.0) * 0.1;
                        info!("ratev {} ratex {}", value.0, base_rate);
                        state.lfo2.set_rate_hz(base_rate.min(40.0).max(0.03));
                        // context.strings.push(format!("{:?}\n{:.2}", param, state.lfo2.get_rate_hz()));
                    }
                    CtlParam::Lfo2Amt => {
                        state.lfo2.set_amount(f32::from(value.0) / f32::from(U7::MAX.0));
                        // context.strings.push(format!("{:?}\n{:.2}", param, state.lfo2.get_amount()));
                    }
                    CtlParam::Lfo2Wave => {
                        state.lfo2.set_waveform(Waveform::from(value.0.min(3)));
                        // context.strings.push(format!("{:?}\n{:?}", param, state.lfo2.get_waveform()));
                    }
                    CtlParam::Lfo2Dest => {
                        if let Some(mod_p) = state.lfo2_param.map(Dw6Param::from) {
                            state.unset_modulated(mod_p).await?;
                        }
                        if let Some(ref mut dump) = &mut state.current_dump {
                            let new_dest = Lfo2Dest::try_from(value.0).ok();
                            if let Some(mod_p) = new_dest.map(Dw6Param::from) {
                                let saved_val = dw6000::get_param_value(mod_p, dump);
                                state.set_modulated(mod_p, saved_val);
                                state.lfo2_param = new_dest;
                            }
                        }
                    }
                }
            }
        _ => {}
    }
    Ok(true)
}

#[derive(Debug, Copy, Clone)]
enum CtlParam {
    Lfo2Rate,
    Lfo2Wave,
    Lfo2Dest,
    Lfo2Amt,
}

fn cc_to_ctl_param(cc: midi::Control, page: KnobPage) -> Option<CtlParam> {
    match page {
        KnobPage::Mod => {
            match cc.into() {
                9 => Some(CtlParam::Lfo2Rate),
                10 => Some(CtlParam::Lfo2Amt),
                11 => Some(CtlParam::Lfo2Wave),
                12 => Some(CtlParam::Lfo2Dest),
                _ => None
            }
        }
        // KnobPage::Arp => {}
        _ => None
    }
}

async fn packets_from_dw_6000(packets: PacketList) {
    for packet in packets.0.into_iter() {
        // lock sysex buffer
        debug!("packet from DW6");
        let mut buffer = DW6_SYSEX_DUMP.lock().await;
        if let Ok(msg) = MidiMessage::try_from(packet) {
            match capture_sysex(buffer.get_mut().unwrap(), msg) {
                Ok(SysexCapture::Captured) =>
                    if let Err(err) = from_dw6000_dump(buffer.get().unwrap()).await {
                        error!("{}", err);
                    }
                Ok(SysexCapture::Pending) => {}
                #[cfg_attr(feature = "defmt", derive(defmt::Format))]
                Err(_err) => warn!("sysex capture error")
            }
        }
    }
}

async fn from_dw6000_dump(dump: &[u8]) -> Result<bool, MidiError> {
    let mut state = DW6_CTRL.lock().await;
    // if let Some(mut dump) = ctx.tags.remove(&Tag::Dump(26)) {
    // rewrite original values before they were modulated
    for s in &state.get_mut().unwrap().mod_dump {
        dw6000::set_param_value(*s.0, *s.1, dump)
    }
    state.get_mut().unwrap().current_dump = Some(Vec::from_slice(dump).unwrap());
    Ok(false)
}

fn param_set_sysex(param: Dw6Param, dump_buf: &[u8]) -> sysex::SysexSeq {
    let dump = dw6000::as_dump_ref(dump_buf);
    let (p, v) = match param {
        Dw6Param::AssignMode | Dw6Param::BendOsc => (0, dump.assign_mode_bend_osc.0),
        Dw6Param::Portamento => (1, dump.portamento_time.0),
        Dw6Param::Osc1Level => (2, dump.osc1_level.0),
        Dw6Param::Osc2Level => (3, dump.osc2_level.0),
        Dw6Param::Noise => (4, dump.noise_level.0),
        Dw6Param::Cutoff => (5, dump.cutoff.0),
        Dw6Param::Resonance => (6, dump.resonance.0),
        Dw6Param::VcfInt => (7, dump.vcf_eg_int.0),
        Dw6Param::VcfAttack => (8, dump.vcf_eg_attack.0),
        Dw6Param::VcfDecay => (9, dump.vcf_eg_decay.0),
        Dw6Param::VcfBreak => (10, dump.vcf_eg_breakpoint.0),
        Dw6Param::VcfSlope => (11, dump.vcf_eg_slope.0),
        Dw6Param::VcfSustain => (12, dump.vcf_eg_sustain.0),
        Dw6Param::VcfRelease => (13, dump.vcf_eg_release.0),
        Dw6Param::VcaAttack => (14, dump.vca_eg_attack.0),
        Dw6Param::VcaDecay => (15, dump.vca_eg_decay.0),
        Dw6Param::VcaBreak => (16, dump.vca_eg_breakpoint.0),
        Dw6Param::VcaSlope => (17, dump.vca_eg_slope.0),
        Dw6Param::BendVcf | Dw6Param::VcaSustain => (18, dump.bend_vcf_vca_eg_sustain.0),
        Dw6Param::Osc1Octave | Dw6Param::VcaRelease => (19, dump.osc1_oct_vca_eg_release.0),
        Dw6Param::Osc2Octave | Dw6Param::MgFreq => (20, dump.osc2_oct_mg_freq.0),
        Dw6Param::KbdTrack | Dw6Param::MgDelay => (21, dump.kbd_track_mg_delay.0),
        Dw6Param::Polarity | Dw6Param::MgOsc => (22, dump.polarity_mg_osc.0),
        Dw6Param::Chorus | Dw6Param::MgVcf => (23, dump.chorus_mg_vcf.0),
        Dw6Param::Osc1Wave | Dw6Param::Osc2Wave => (24, dump.osc1_wave_osc2_wave.0),
        Dw6Param::Osc2Detune | Dw6Param::Interval => (25, dump.osc2_interval_osc2_detune.0),
    };
    dw6000::set_parameter_sysex(p, v)
}

fn cc_to_dw_param(cc: midi::Control, page: KnobPage) -> Option<Dw6Param> {
    match cc.into() {
        // jogwheel hardwired to cutoff for her pleasure
        17 => return Some(Dw6Param::Cutoff),
        8 => return Some(Dw6Param::Resonance),
        // AssignMode => defined on DW6000 panel
        18 => return Some(Dw6Param::Polarity),
        19 => return Some(Dw6Param::Chorus),

        _ => {}
    }

    match page {
        KnobPage::Osc => match cc.into() {
            1 => Some(Dw6Param::Osc1Level),
            2 => Some(Dw6Param::Osc1Octave),
            3 => Some(Dw6Param::Osc1Wave),
            4 => Some(Dw6Param::Noise),
            5 => Some(Dw6Param::BendOsc),
            6 => Some(Dw6Param::BendVcf),
            7 => Some(Dw6Param::Portamento),

            9 => Some(Dw6Param::Osc2Level),
            10 => Some(Dw6Param::Osc2Octave),
            11 => Some(Dw6Param::Osc2Wave),
            12 => Some(Dw6Param::Interval),
            13 => Some(Dw6Param::Osc2Detune),
            // 14 => Some(Param::Osc2Wave),
            _ => None
        }
        KnobPage::Env => match cc.into() {
            1 => Some(Dw6Param::VcaAttack),
            2 => Some(Dw6Param::VcaDecay),
            3 => Some(Dw6Param::VcaBreak),
            4 => Some(Dw6Param::VcaSustain),
            5 => Some(Dw6Param::VcaSlope),
            6 => Some(Dw6Param::VcaRelease),

            9 => Some(Dw6Param::VcfAttack),
            10 => Some(Dw6Param::VcfDecay),
            11 => Some(Dw6Param::VcfBreak),
            12 => Some(Dw6Param::VcfSustain),
            13 => Some(Dw6Param::VcfSlope),
            14 => Some(Dw6Param::VcfRelease),
            15 => Some(Dw6Param::VcfInt),
            16 => Some(Dw6Param::KbdTrack),
            _ => None,
        }
        KnobPage::Mod => match cc.into() {
            1 => Some(Dw6Param::MgFreq),
            2 => Some(Dw6Param::MgDelay),
            3 => Some(Dw6Param::MgOsc),
            4 => Some(Dw6Param::MgVcf),
            5 => Some(Dw6Param::BendOsc),
            6 => Some(Dw6Param::BendVcf),
            7 => Some(Dw6Param::Portamento),
            // TODO LFO2 (? - Rate, Sync, Shape, Amt, Target)
            _ => None,
        }
        KnobPage::Arp => match cc.into() {
            // TODO Arp control (Rate, Oct, Mode, Order)
            0 => None,
            _ => None,
        }
    }
}
