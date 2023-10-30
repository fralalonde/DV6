#![no_std]

use core::array::TryFromSliceError;
use core::iter::FromIterator;
use core::ops::{Deref, DerefMut};

use heapless::Vec;
#[cfg(feature = "usb")]
use usb_device::UsbError;

pub use message::{MidiMessage, note_off, note_on, program_change};
pub use note::Note;
pub use packet::{CableNumber, CodeIndexNumber, Packet};

pub use status::Status;
pub use u14::U14;
pub use u4::U4;
pub use u6::U6;
pub use u7::U7;
pub use parser::{PacketParser};
pub use status::{is_non_status, is_channel_status, StatusPacker};
pub use sysex::{capture_sysex, SysexCapture, SysexError};

mod u4;
mod u6;
mod u7;
mod u14;
mod status;
mod note;
mod message;
mod packet;
mod parser;
mod sysex;

use num_enum::{TryFromPrimitive, };

/// MIDI channel, stored as 0-15
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(num_enum::TryFromPrimitive, num_enum::UnsafeFromPrimitive, num_enum::IntoPrimitive)]
pub enum MidiChannel {
    CH1,
    CH2,
    CH3,
    CH4,
    CH5,
    CH6,
    CH7,
    CH8,
    CH9,
    CH10,
    CH11,
    CH12,
    CH13,
    CH14,
    CH15,
    CH16,
}

/// "Natural" channel builder, takes integers 1-16 as input
/// panics if channel is outside of range
pub fn channel(ch: impl Into<u8>) -> Result<MidiChannel, MidiError> {
    let ch = ch.into();
    MidiChannel::try_from_primitive(ch - 1).map_err(|_err| MidiError::InvalidChannel)
}

impl MidiChannel {
    pub fn as_u8(&self) -> u8 {
        (*self).into()
    }
}

pub type Velocity = U7;
pub type Control = U7;
pub type Pressure = U7;
pub type Program = U7;
pub type Bend = U14;

const MAX_PACKETS: usize = 16;

#[derive(Default, Debug, Clone)]
pub struct PacketList(pub Vec<Packet, MAX_PACKETS>);

impl Deref for PacketList {
    type Target = Vec<Packet, MAX_PACKETS>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PacketList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Packet> for PacketList {
    fn from_iter<T: IntoIterator<Item=Packet>>(iter: T) -> Self {
        let mut list = Vec::new();
        for p in iter {
            if list.push(p).is_err() {
                break;
            }
        }
        PacketList(list)
    }
}

impl PacketList {
    pub fn single(packet: Packet) -> Self {
        let mut list = Vec::new();
        let _ = list.push(packet);
        PacketList(list)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MidiError {
    SysexInterrupted,
    InvalidStatus(u8),
    BadPacket(Packet),
    NoModeForParameter,
    SysexOutOfBounds,
    InvalidCodeIndexNumber,
    InvalidCableNumber,
    InvalidChannel,
    InvalidProgram,
    InvalidNote,
    InvalidVelocity,
    InvalidInteger,

    // External errors
    TryFromSliceError,
    PortError,
    WriteError,
    ReadError,
    BufferFull,
    TooManyPorts,
    InvalidPort,
    DroppedPacket,
    // UnknownInterface(MidiInterface),

    #[cfg(feature = "embassy-stm32")]
    Stm32UsartError,
}

#[cfg(feature = "usb")]
impl From<UsbError> for MidiError {
    fn from(_err: UsbError) -> Self {
        MidiError::PortError
    }
}

impl<E> From<nb::Error<E>> for MidiError {
    fn from(_: nb::Error<E>) -> Self {
        MidiError::PortError
    }
}

/// RTIC spawn error
impl From<TryFromSliceError> for MidiError {
    fn from(_: TryFromSliceError) -> Self {
        MidiError::TryFromSliceError
    }
}

#[cfg(feature = "embassy-stm32")]
impl From<embassy_stm32::usart::Error> for MidiError {
    fn from(_err: embassy_stm32::usart::Error) -> Self {
        MidiError::Stm32UsartError
    }
}

/// Just strip higher bits (meh)
pub trait Cull<T>: Sized {
    fn cull(_: T) -> Self;
}

/// Saturate to T::MAX
pub trait Fill<T>: Sized {
    fn fill(_: T) -> Self;
}
