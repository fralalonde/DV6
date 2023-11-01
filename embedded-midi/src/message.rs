use core::convert::{TryFrom, TryInto};
use MidiMessage::*;
use CodeIndexNumber::{SystemCommonLen1, SystemCommonLen2, SystemCommonLen3};

use crate::{MidiChannel, Note, Velocity, Pressure, Program, Control, U7, Bend, CodeIndexNumber, Packet, Status, MidiError, Cull};
use crate::status::{SYSEX_END, is_non_status, SYSEX_START, NOTE_OFF, NOTE_ON, NOTE_PRESSURE, CHANNEL_PRESSURE, PROGRAM_CHANGE, CONTROL_CHANGE, PITCH_BEND, TIME_CODE_QUARTER_FRAME, SONG_POSITION_POINTER, SONG_SELECT, TUNE_REQUEST, TIMING_CLOCK, START, CONTINUE, STOP, ACTIVE_SENSING, SYSTEM_RESET, MEASURE_END};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(unused)]
pub enum MidiMessage {
    NoteOff(MidiChannel, Note, Velocity),
    NoteOn(MidiChannel, Note, Velocity),

    NotePressure(MidiChannel, Note, Pressure),
    ChannelPressure(MidiChannel, Pressure),
    ProgramChange(MidiChannel, Program),
    ControlChange(MidiChannel, Control, U7),
    PitchBend(MidiChannel, Bend),

    // System
    TimeCodeQuarterFrame(U7),
    SongPositionPointer(U7, U7),
    SongSelect(U7),
    TuneRequest,

    // System Realtime
    TimingClock,
    MeasureEnd(U7),
    Start,
    Continue,
    Stop,
    ActiveSensing,
    SystemReset,

    // Sysex
    SysexBegin(u8, u8),
    SysexCont(u8, u8, u8),
    SysexEnd,
    SysexEnd1(u8),
    SysexEnd2(u8, u8),

    // "special cases" - as per the USB MIDI spec
    SysexEmpty,
    SysexSingleByte(u8),
}

impl MidiMessage {
    pub fn status_byte(&self) -> Option<u8> {
        match self {
            MidiMessage::NoteOff(ch, ..) => Some(NOTE_OFF + ch.as_u8()),
            MidiMessage::NoteOn(ch, ..) => Some(NOTE_ON + ch.as_u8()),
            MidiMessage::NotePressure(ch, ..) => Some(NOTE_PRESSURE + ch.as_u8()),
            MidiMessage::ChannelPressure(ch, ..) => Some(CHANNEL_PRESSURE + ch.as_u8()),
            MidiMessage::ProgramChange(ch, ..) => Some(PROGRAM_CHANGE + ch.as_u8()),
            MidiMessage::ControlChange(ch, ..) => Some(CONTROL_CHANGE + ch.as_u8()),
            MidiMessage::PitchBend(ch, ..) => Some(PITCH_BEND + ch.as_u8()),

            MidiMessage::TimeCodeQuarterFrame(_) => Some(TIME_CODE_QUARTER_FRAME),
            MidiMessage::SongPositionPointer(_, _) => Some(SONG_POSITION_POINTER),
            MidiMessage::SongSelect(_) => Some(SONG_SELECT),
            MidiMessage::TuneRequest => Some(TUNE_REQUEST),
            MidiMessage::TimingClock => Some(TIMING_CLOCK),
            MidiMessage::Start => Some(START),
            MidiMessage::Continue => Some(CONTINUE),
            MidiMessage::Stop => Some(STOP),
            MidiMessage::ActiveSensing => Some(ACTIVE_SENSING),
            MidiMessage::SystemReset => Some(SYSTEM_RESET),
            MidiMessage::MeasureEnd(_) => Some(MEASURE_END),
            _ => None,
        }
    }
}

pub fn note_on(channel: MidiChannel, note: impl TryInto<Note>, velocity: impl TryInto<Velocity>) -> Result<MidiMessage, MidiError> {
    Ok(NoteOn(
        channel,
        note.try_into().map_err(|_| MidiError::InvalidNote)?,
        velocity.try_into().map_err(|_| MidiError::InvalidVelocity)?)
    )
}

pub fn note_off(channel: MidiChannel, note: impl TryInto<Note>, velocity: impl TryInto<Velocity>) -> Result<MidiMessage, MidiError> {
    Ok(NoteOff(
        channel,
        note.try_into().map_err(|_| MidiError::InvalidNote)?,
        velocity.try_into().map_err(|_| MidiError::InvalidVelocity)?)
    )
}

pub fn program_change(channel: MidiChannel, program: impl TryInto<Program>) -> Result<MidiMessage, MidiError> {
    Ok(ProgramChange(
        channel,
        program.try_into().map_err(|_| MidiError::InvalidProgram)?,
    ))
}


impl TryFrom<Packet> for MidiMessage {
    type Error = MidiError;

    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        match (packet.code_index_number(), packet.status(), packet.channel(), &packet.bytes()[1..4]) {
            (CodeIndexNumber::Sysex, _, _, payload) => {
                if is_non_status(payload[0]) {
                    Ok(SysexCont(payload[0], payload[1], payload[2]))
                } else {
                    Ok(SysexBegin(payload[1], payload[2]))
                }
            }
            (SystemCommonLen1, _, _, payload) if payload[0] == SYSEX_END => Ok(SysexEnd),
            (CodeIndexNumber::SysexEndsNext2, _, _, payload) => {
                if payload[0] == SYSEX_START {
                    Ok(SysexEmpty)
                } else {
                    Ok(SysexEnd1(payload[0]))
                }
            }
            (CodeIndexNumber::SysexEndsNext3, _, _, payload) => {
                if payload[0] == SYSEX_START {
                    Ok(SysexSingleByte(payload[1]))
                } else {
                    Ok(SysexEnd2(payload[0], payload[1]))
                }
            }

            (SystemCommonLen1, Ok(Some(Status::TimingClock)), ..) => Ok(TimingClock),
            (SystemCommonLen1, Ok(Some(Status::TuneRequest)), ..) => Ok(TuneRequest),
            (SystemCommonLen1, Ok(Some(Status::Start)), ..) => Ok(Start),
            (SystemCommonLen1, Ok(Some(Status::Continue)), ..) => Ok(Continue),
            (SystemCommonLen1, Ok(Some(Status::Stop)), ..) => Ok(Stop),
            (SystemCommonLen1, Ok(Some(Status::ActiveSensing)), ..) => Ok(ActiveSensing),
            (SystemCommonLen1, Ok(Some(Status::SystemReset)), ..) => Ok(SystemReset),
            (SystemCommonLen2, Ok(Some(Status::TimeCodeQuarterFrame)), _, payload) => Ok(TimeCodeQuarterFrame(U7::cull(payload[1]))),
            (SystemCommonLen2, Ok(Some(Status::SongSelect)), _, payload) => Ok(SongSelect(U7::cull(payload[1]))),
            (SystemCommonLen2, Ok(Some(Status::MeasureEnd)), _, payload) => Ok(MeasureEnd(U7::cull(payload[1]))),
            (SystemCommonLen3, Ok(Some(Status::SystemReset)), _, payload) => Ok(SongPositionPointer(U7::cull(payload[1]), U7::cull(payload[1]))),

            (_, Ok(Some(Status::NoteOff)), Some(channel), payload) => Ok(NoteOff(channel, Note::try_from(payload[1])?, Velocity::try_from(payload[2])?)),
            (_, Ok(Some(Status::NoteOn)), Some(channel), payload) => Ok(NoteOn(channel, Note::try_from(payload[1])?, Velocity::try_from(payload[2])?)),
            (_, Ok(Some(Status::NotePressure)), Some(channel), payload) => Ok(NotePressure(channel, Note::try_from(payload[1])?, Pressure::try_from(payload[2])?)),
            (_, Ok(Some(Status::ChannelPressure)), Some(channel), payload) => Ok(ChannelPressure(channel, Pressure::try_from(payload[1])?)),
            (_, Ok(Some(Status::ProgramChange)), Some(channel), payload) => Ok(ProgramChange(channel, U7::try_from(payload[1])?)),
            (_, Ok(Some(Status::ControlChange)), Some(channel), payload) => Ok(ControlChange(channel, Control::try_from(payload[1])?, U7::try_from(payload[2])?)),
            (_, Ok(Some(Status::PitchBend)), Some(channel), payload) => Ok(PitchBend(channel, Bend::try_from((payload[1], payload[2]))?)),

            (..) => Err(MidiError::BadPacket(packet)),
        }
    }
}
