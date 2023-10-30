use heapless::Vec;
use crate::MidiMessage;
use crate::MidiMessage::*;

pub enum SysexCapture {
    Captured(usize),
    Pending(usize),
    NotSysex,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SysexError {
    BufferOverflow,
    SpuriousContinuation,
    SpuriousEnd,
}

impl From<u8> for SysexError {
    fn from(_value: u8) -> Self {
        SysexError::BufferOverflow
    }
}

pub fn capture_sysex<const N: usize>(buffer: &mut Vec<u8, N>, message: MidiMessage) -> Result<SysexCapture, SysexError> {
    match message {
        SysexBegin(byte0, byte1) => {
            buffer.clear();
            buffer.push(byte0)?;
            buffer.push(byte1)?;
            Ok(SysexCapture::Pending(buffer.len()))
        }
        SysexSingleByte(byte0) => {
            buffer.clear();
            buffer.push(byte0)?;
            Ok(SysexCapture::Captured(buffer.len()))
        }
        SysexEmpty => {
            buffer.clear();
            Ok(SysexCapture::Captured(buffer.len()))
        }
        SysexCont(byte0, byte1, byte2) => {
            if buffer.is_empty() {
                // there should be _some_ data buffered from previous messages
                return Err(SysexError::SpuriousContinuation);
            }
            buffer.push(byte0)?;
            buffer.push(byte1)?;
            buffer.push(byte2)?;
            Ok(SysexCapture::Pending(buffer.len()))
        }
        SysexEnd => {
            if buffer.is_empty() {
                // there should be _some_ data buffered from previous messages
                return Err(SysexError::SpuriousEnd);
            }
            Ok(SysexCapture::Captured(buffer.len()))
        }
        SysexEnd1(byte0) => {
            if buffer.is_empty() {
                return Err(SysexError::SpuriousEnd);
            }
            buffer.push(byte0)?;
            Ok(SysexCapture::Captured(buffer.len()))
        }
        SysexEnd2(byte0, byte1) => {
            if buffer.is_empty() {
                return Err(SysexError::SpuriousEnd);
            }
            buffer.push(byte0)?;
            buffer.push(byte1)?;
            Ok(SysexCapture::Captured(buffer.len()))
        }
        _ => {
            // message is not part of a sysex sequence
            buffer.clear();
            Ok(SysexCapture::NotSysex)
        }
    }
}