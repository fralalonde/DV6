//! MIDI using HAL Serial

use embassy_stm32::usart::{BasicInstance, BufferedUartRx, BufferedUartTx};
use embedded_io_async::{Write, Read};
use midi::{Packet, MidiError, PacketList, StatusPacker};

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
