//! MIDI using HAL Serial

use embassy_stm32::dma::NoDma;
use embassy_stm32::usart::{BasicInstance, BufferedUartRx, BufferedUartTx, UartRx, UartTx};
use embedded_io_async::{Write, Read};
use futures::TryFutureExt;

use midi::{Packet, MidiError, PacketList};

#[derive(Default)]
pub struct StatusPacker {
    last_status: Option<u8>,
}

impl StatusPacker {
    pub fn pack<'a>(&mut self, payload: &'a [u8]) -> &'a [u8] {
        if midi::is_channel_status(payload[0]) {
            // Apply MIDI "running status"
            if let Some(last_status) = self.last_status {
                if payload[0] == last_status {
                    // same status as last time, chop out status byte
                    return &payload[1..];
                } else {
                    // take note of new status
                    self.last_status = Some(payload[0])
                }
            }
        } else {
            // non-repeatable status or no status (sysex)
            self.last_status = None
        }
        payload
    }
}

pub struct BufferedSerialMidiOut<'a, UART: BasicInstance> {
    pub uart: BufferedUartTx<'a, UART>,
    packer: StatusPacker,
}

impl<'a, UART: BasicInstance> BufferedSerialMidiOut<'a, UART> where UART: {
    pub fn new(uart: BufferedUartTx<'a, UART>) -> Self {
        Self {
            uart,
            packer: StatusPacker::default(),
        }
    }
    pub(crate) async fn transmit(&mut self, packets: PacketList) -> Result<(), MidiError> {
        for packet in packets.iter() {
            let payload = packet.payload();
            let payload = self.packer.pack(payload);
            self.uart.write_all(payload).await?;
        }
        Ok(())
    }
}

pub struct SerialMidiOut<'a, UART: BasicInstance, TxDma = NoDma> {
    pub uart: UartTx<'a, UART, TxDma>,
    packer: StatusPacker,
}

impl<'a, UART: BasicInstance, TxDma: embassy_stm32::usart::TxDma<UART>> SerialMidiOut<'a, UART, TxDma> where UART: {
    pub fn new(uart: UartTx<'a, UART, TxDma>) -> Self {
        Self {
            uart,
            packer: StatusPacker::default(),
        }
    }
    pub(crate) async fn transmit(&mut self, packets: PacketList) -> Result<(), MidiError> {
        for packet in packets.iter() {
            let payload = packet.payload();
            let payload = self.packer.pack(payload);
            self.uart.write(payload).await?;
        }
        Ok(())
    }
}

pub struct SerialMidiIn<'a, UART: BasicInstance, RxDma = NoDma> {
    pub uart: UartRx<'a, UART, RxDma>,
    parser: midi::PacketParser,
}

impl<'a, UART: BasicInstance, RxDma: embassy_stm32::usart::RxDma<UART>> SerialMidiIn<'a, UART, RxDma> {
    pub fn new(uart: UartRx<'a, UART, RxDma>) -> Self {
        Self {
            uart,
            parser: midi::PacketParser::default(),
        }
    }

    pub async fn receive(&mut self) -> Result<Packet, MidiError> {
        let mut z: [u8; 1] = [0];
        loop {
            // no size check - successful async read() guarantees buffer was filled :shrug:
            self.uart.read(&mut z).await?;
            let packet = self.parser.advance(z[0])?;
            if let Some(packet) = packet {
                return Ok(packet.with_cable_num(1));
            }
        }
    }
}

pub struct BufferedSerialMidiIn<'a, UART: BasicInstance> {
    pub uart: BufferedUartRx<'a, UART>,
    parser: midi::PacketParser,
}

impl<'a, UART: BasicInstance> BufferedSerialMidiIn<'a, UART> {
    pub fn new(uart: BufferedUartRx<'a, UART>) -> Self {
        Self {
            uart,
            parser: midi::PacketParser::default(),
        }
    }

    pub async fn receive(&mut self) -> Result<Packet, MidiError> {
        let mut z: [u8; 1] = [0];
        loop {
            // no size check - successful async read() guarantees buffer was filled :shrug:
            if self.uart.read(&mut z).await? == 1 {
                let packet = self.parser.advance(z[0])?;
                if let Some(packet) = packet {
                    return Ok(packet.with_cable_num(1));
                }
            }
        }
    }
}
