
use midi::{Packet, MidiMessage, PacketList};

use core::iter::FromIterator;
use heapless::Vec;

/// Used to send sysex
/// Accepts same Token as matcher for convenience, but only Match and Val value are sent
#[derive(Debug)]
pub struct SysexSeq<const N: usize> {
    // TODO stream from u8 iterator
    bytes: Vec<u8, N>,
    pos: usize,
    done: bool,
}

impl<const N: usize> SysexSeq<N> {
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    pub fn from_vec(bytes: Vec<u8, N>) -> Self {
        SysexSeq {
            bytes,
            pos: 0,
            done: false,
        }
    }

    pub fn from_slices(slices: &[&[u8]]) -> Self {
        let mut new = Self::new();
        new.extend_from_slices(slices);
        new
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.bytes.extend_from_slice(slice).unwrap()
    }

    pub fn extend_from_slices(&mut self, slices: &[&[u8]]) {
        for s in slices {
            self.extend_from_slice(s)
        }
    }
}

impl<const N: usize> From<SysexSeq<N>> for PacketList {
    fn from(value: SysexSeq<N>) -> Self {
        PacketList::from_iter(value)
    }
}

impl<const N: usize> Iterator for SysexSeq<N> {
    type Item = Packet;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        use MidiMessage::*;
        Some(Packet::from(match (self.pos, self.bytes.len(), self.bytes.len() - self.pos) {
            (0, 0, _) => {self.done = true; SysexEmpty},
            (0, 1, _) => {self.done = true; SysexSingleByte(self.bytes[0])},
            (0, _, _) => { self.pos += 2; SysexBegin(self.bytes[self.pos - 2], self.bytes[self.pos - 1]) },

            (_, _, 0) => { self.done = true; SysexEnd },
            (_, _, 1) => { self.done = true; SysexEnd1(self.bytes[self.pos]) },
            (_, _, 2) => { self.done = true; SysexEnd2(self.bytes[self.pos], self.bytes[self.pos + 1]) },

            (..) => { self.pos += 3; SysexCont(self.bytes[self.pos - 3], self.bytes[self.pos - 2], self.bytes[self.pos - 1]) },
        }))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ExpType {
    Channel,
    Velocity,
    DeviceId,
    /// Parameter code (cutoff, delay, etc.)
    ParamId,
    /// Control code (knob, pad, etc.)
    ControlId,
    /// Value of parameter
    ValueU7,
    /// Value of parameter
    MsbValueU4,
    /// Value of parameter
    LsbValueU4,
    /// Raw data
    Bytes(usize),
}

impl ExpType {
    pub fn len(&self) -> usize {
        match self {
            ExpType::Bytes(len) => *len,
            _ => 1,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum PatternExp {
    Skip(usize),
    Val(u8),
    Seq(&'static [u8]),
    Cap(ExpType),
}

pub fn pattern_match<const N: usize>(sysex_buffer: &[u8], pattern: &[PatternExp], captured: &mut Vec<(usize, ExpType), N>) -> bool {
    let mut pos = 0;

    let mut buffer = sysex_buffer;

    for exp in pattern {
        if pos > sysex_buffer.len() { return false; }
        buffer = &sysex_buffer[pos..];
        match exp {
            PatternExp::Skip(len) => pos += len,
            PatternExp::Val(token) => if *token == buffer[0] { pos += 1 } else { return false; }
            PatternExp::Seq(seq) =>
                if buffer.starts_with(seq) { pos += seq.len() } else { return false; }
            PatternExp::Cap(exp_type) => {
                if pos + exp_type.len() > buffer.len() { return false; }
                captured.push((pos, *exp_type)).expect("sysex capture buffer overflow");
                pos += exp_type.len();
            }
        };
    }
    // check that no byte remains unmatched
    pos == sysex_buffer.len()
}

