//! MIDI using HAL Serial

use embassy_stm32::dma::NoDma;
use embassy_stm32::usart::{BasicInstance, UartRx, UartTx};
use embedded_io_async::{Write, Read};
use futures::TryFutureExt;

use midi::{Packet, MidiError, PacketList};

pub struct SerialMidiOut<'a, UART: BasicInstance, TxDma = NoDma> {
    pub uart: UartTx<'a, UART, TxDma>,
    last_status: Option<u8>,
}

impl<'a, UART: BasicInstance, TxDma> SerialMidiOut<'a, UART, TxDma> {
    pub fn new(uart: UartTx<'a, UART, TxDma>) -> Self {
        Self {
            uart,
            last_status: None,
        }
    }
}

impl<'a, UART: BasicInstance, TxDma: embassy_stm32::usart::TxDma<UART>> SerialMidiOut<'a, UART, TxDma> where UART: {
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
            self.uart.write(payload).map_err(|_| MidiError::WriteError).await?;
        }
        Ok(())
    }
}

pub struct SerialMidiIn<'a, UART: BasicInstance, RxDma = NoDma> {
    pub uart: UartRx<'a, UART, RxDma>,
    parser: midi::PacketParser,
}

impl<'a, UART: BasicInstance, RxDma> SerialMidiIn<'a, UART, RxDma> {
    pub fn new(uart: UartRx<'a, UART, RxDma>) -> Self {
        Self {
            uart,
            parser: midi::PacketParser::default(),
        }
    }
}

impl<'a, UART: BasicInstance, RxDma: embassy_stm32::usart::RxDma<UART>> SerialMidiIn<'a, UART, RxDma> {
    pub async fn receive(&mut self) -> Result<Packet, MidiError> {
        let mut z: [u8; 1] = [0];
        loop {
            // no size check - successful async read() guarantees buffer was filled :shrug:
            self.uart.read(&mut z).await.map_err(|_| MidiError::ReadError)?;
            let packet = self.parser.advance(z[0])?;
            if let Some(packet) = packet {
                return Ok(packet.with_cable_num(1));
            }
        }
    }
}




