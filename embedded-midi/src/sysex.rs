use heapless::Vec;
use crate::MidiMessage;
use crate::MidiMessage::*;

pub enum SysexCapture {
    Captured,
    Pending,
}

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
            Ok(SysexCapture::Pending)
        }
        SysexSingleByte(byte0) => {
            buffer.clear();
            buffer.push(byte0)?;
            Ok(SysexCapture::Captured)
        }
        SysexEmpty => {
            buffer.clear();
            Ok(SysexCapture::Captured)
        }
        SysexCont(byte0, byte1, byte2) => {
            if buffer.is_empty() {
                // there should be _some_ data buffered from previous messages
                return Err(SysexError::SpuriousContinuation);
            }
            buffer.push(byte0)?;
            buffer.push(byte1)?;
            buffer.push(byte2)?;
            Ok(SysexCapture::Captured)
        }
        SysexEnd => {
            if buffer.is_empty() {
                // there should be _some_ data buffered from previous messages
                return Err(SysexError::SpuriousEnd);
            }
            Ok(SysexCapture::Captured)
        }
        SysexEnd1(byte0) => {
            if buffer.is_empty() {
                return Err(SysexError::SpuriousEnd);
            }
            buffer.push(byte0)?;
            Ok(SysexCapture::Captured)
        }
        SysexEnd2(byte0, byte1) => {
            if buffer.is_empty() {
                return Err(SysexError::SpuriousEnd);
            }
            buffer.push(byte0)?;
            buffer.push(byte1)?;
            Ok(SysexCapture::Captured)
        }
        _ => {
            // message is not part a sysex sequence
            buffer.clear();
            Ok(SysexCapture::Pending)
        }
    }
}