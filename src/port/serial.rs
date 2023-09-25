//! MIDI using HAL Serial

use embassy_stm32::usart::{BasicInstance, BufferedUartRx, BufferedUartTx};
use embedded_io_async::{Write, Read};
use futures::TryFutureExt;

use midi::{Packet, MidiError, PacketList};

pub struct SerialMidiOut<'a, UART: BasicInstance> {
    pub uart: BufferedUartTx<'a, UART>,
    last_status: Option<u8>,
}

impl<'a, UART: BasicInstance> SerialMidiOut<'a, UART> {
    pub fn new(uart: BufferedUartTx<'a, UART>) -> Self {
        Self {
            uart,
            last_status: None,
        }
    }
}

impl<'a, UART: BasicInstance> SerialMidiOut<'a, UART> where UART: {
    pub(crate) async fn transmit(&mut self, packets: PacketList) -> Result<(), MidiError> {
        for packet in packets.iter() {
            let mut payload = packet.payload();

            if midi::is_channel_status(payload[0]) {
                // Apply MIDI "running status"
                if let Some(last_status) = self.last_status {
                    if payload[0] == last_status {
                        // same status as last time, chop out status byte
                        payload = &payload[1..];
                    } else {
                        // take note of new status
                        self.last_status = Some(payload[0])
                    }
                }
            } else {
                // non-repeatable status or no status (sysex)
                self.last_status = None
            }
            self.uart.write_all(payload).map_err(|_| MidiError::WriteError).await?;
        }
        Ok(())
    }
}

pub struct SerialMidiIn<'a, UART: BasicInstance> {
    pub uart: BufferedUartRx<'a, UART>,
    parser: midi::PacketParser,
}

impl<'a, UART: BasicInstance> SerialMidiIn<'a, UART> {
    pub fn new(uart: BufferedUartRx<'a, UART>) -> Self {
        Self {
            uart,
            parser: midi::PacketParser::default(),
        }
    }
}

impl<'a, UART: BasicInstance> SerialMidiIn<'a, UART> {
    pub async fn receive(&mut self) -> Result<Option<Packet>, MidiError> {
        let mut z: [u8; 1] = [0];
        if self.uart.read(&mut z).await.map_err(|_| MidiError::ReadError)? == 1 {
            let packet = self.parser.advance(z[0])?;
            if let Some(packet) = packet {
                return Ok(Some(packet.with_cable_num(1)));
            }
        }
        Ok(None)
    }
}


