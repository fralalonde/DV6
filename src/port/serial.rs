//! MIDI using HAL Serial

use embassy_stm32::dma::NoDma;
use embassy_stm32::usart::{BasicInstance, UartRx, UartTx};
use midi::{Packet, MidiError, PacketList, StatusPacker};

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
