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
        if let Some(status) = packet.status()? {
            if let Some(channel) = packet.channel() {
                match (status, channel, packet.payload()) {
                    (Status::NoteOff, channel, payload) => Ok(NoteOff(channel, Note::try_from(payload[1])?, Velocity::try_from(payload[2])?)),
                    (Status::NoteOn, channel, payload) => Ok(NoteOn(channel, Note::try_from(payload[1])?, Velocity::try_from(payload[2])?)),
                    (Status::NotePressure, channel, payload) => Ok(NotePressure(channel, Note::try_from(payload[1])?, Pressure::try_from(payload[2])?)),
                    (Status::ChannelPressure, channel, payload) => Ok(ChannelPressure(channel, Pressure::try_from(payload[1])?)),
                    (Status::ProgramChange, channel, payload) => Ok(ProgramChange(channel, U7::try_from(payload[1])?)),
                    (Status::ControlChange, channel, payload) => Ok(ControlChange(channel, Control::try_from(payload[1])?, U7::try_from(payload[2])?)),
                    (Status::PitchBend, channel, payload) => Ok(PitchBend(channel, Bend::try_from((payload[1], payload[2]))?)),
                    (..) => Err(MidiError::BadPacket(packet)),
                }
            } else {
                match (packet.code_index_number(), status, packet.payload()) {
                    (SystemCommonLen1, Status::TimingClock, ..) => Ok(TimingClock),
                    (SystemCommonLen1, Status::TuneRequest, ..) => Ok(TuneRequest),
                    (SystemCommonLen1, Status::Start, ..) => Ok(Start),
                    (SystemCommonLen1, Status::Continue, ..) => Ok(Continue),
                    (SystemCommonLen1, Status::Stop, ..) => Ok(Stop),
                    (SystemCommonLen1, Status::ActiveSensing, ..) => Ok(ActiveSensing),
                    (SystemCommonLen1, Status::SystemReset, ..) => Ok(SystemReset),
                    (SystemCommonLen2, Status::TimeCodeQuarterFrame, payload) => Ok(TimeCodeQuarterFrame(U7::cull(payload[1]))),
                    (SystemCommonLen2, Status::SongSelect, payload) => Ok(SongSelect(U7::cull(payload[1]))),
                    (SystemCommonLen2, Status::MeasureEnd, payload) => Ok(MeasureEnd(U7::cull(payload[1]))),
                    (SystemCommonLen3, Status::SystemReset, payload) => Ok(SongPositionPointer(U7::cull(payload[1]), U7::cull(payload[1]))),
                    (..) => Err(MidiError::BadPacket(packet)),
                }
            }
        } else {
            match (packet.code_index_number(), packet.payload()) {
                (CodeIndexNumber::Sysex, payload) => {
                    if is_non_status(payload[0]) {
                        Ok(SysexCont(payload[0], payload[1], payload[2]))
                    } else {
                        Ok(SysexBegin(payload[1], payload[2]))
                    }
                }
                (SystemCommonLen1, payload) if payload[0] == SYSEX_END => Ok(SysexEnd),
                (CodeIndexNumber::SysexEndsNext2,  payload) => {
                    if payload[0] == SYSEX_START {
                        Ok(SysexEmpty)
                    } else {
                        Ok(SysexEnd1(payload[0]))
                    }
                }
                (CodeIndexNumber::SysexEndsNext3, payload) => {
                    if payload[0] == SYSEX_START {
                        Ok(SysexSingleByte(payload[1]))
                    } else {
                        Ok(SysexEnd2(payload[0], payload[1]))
                    }
                }
                (..) => Err(MidiError::BadPacket(packet)),
            }
        }
    }
}
