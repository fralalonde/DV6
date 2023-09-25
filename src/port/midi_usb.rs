use core::mem;
use core::mem::MaybeUninit;
use core::result::Result;
use embassy_usb::driver::{Driver, Endpoint, EndpointError, EndpointIn, EndpointOut};
use embassy_usb::types::{InterfaceNumber};
use embassy_usb::{Builder, Handler};

use midi::{Packet};


pub const USB_MIDI_OUT_SIZE: u8 = 0x09;

pub const USB_CLASS_NONE: u8 = 0x00;
pub const USB_AUDIO_CLASS: u8 = 0x01;
pub const USB_AUDIO_CONTROL_SUBCLASS: u8 = 0x01;

pub const USB_MIDI_IN_JACK_SUBTYPE: u8 = 0x02;
pub const USB_MIDI_OUT_JACK_SUBTYPE: u8 = 0x03;

pub const CS_INTERFACE: u8 = 0x24;

pub const USB_HEADER_SUBTYPE: u8 = 0x01;
pub const USB_MS_HEADER_SUBTYPE: u8 = 0x01;


pub struct MidiClass<'d, D: Driver<'d>> {
    // comm_ep: D::EndpointIn,

    data_if: InterfaceNumber,
    read_ep: D::EndpointOut,
    write_ep: D::EndpointIn,

    max_packet_size: usize,
}

const USB_AUDIO_PROTOCOL_NONE: u8 = 0x00;

const USB_CLASS_AUDIO: u8 = 0x01;
const AUDIO_SUBCLASS_CONTROL: u8 = 0x01;
const AUDIO_SUBCLASS_MS: u8 = 0x03;

const MS_TYPE_GENERAL: u8 = 0x01;

const CS_ENDPOINT: u8 = 0x25;

const MS_MIDI_IN_JACK: u8 = 0x02;
const MS_MIDI_OUT_JACK: u8 = 0x03;

impl<'d, D: Driver<'d>> MidiClass<'d, D> {
    /// Create a new MIDI class.
    pub fn new(
        builder: &mut Builder<'d, D>,
        state: &'d mut State<'d>,
        max_packet_size: u16,
    ) -> Self {
        let mut func = builder.function(USB_AUDIO_CLASS, USB_AUDIO_CONTROL_SUBCLASS, USB_AUDIO_PROTOCOL_NONE);

        // Control interface
        let mut audio_if = func.interface();

        // AC interface
        let mut config = audio_if.alt_setting(USB_AUDIO_CLASS, USB_AUDIO_CONTROL_SUBCLASS, USB_AUDIO_PROTOCOL_NONE, None);
        let data_if = config.interface_number();

        config.descriptor(
            CS_INTERFACE,
            &[
                USB_HEADER_SUBTYPE,
                0x00,
                0x01, // Revision
                0x09,
                0x00, // SIZE of class specific descriptions
                0x01, // Number of streaming interfaces
                0x01, // MIDI Streaming interface 1 belongs to this AC interface
            ],
        );

        // MS interface
        config.descriptor(
            CS_INTERFACE,
            &[USB_CLASS_AUDIO, AUDIO_SUBCLASS_MS, USB_CLASS_NONE],
        );

        config.descriptor(
            CS_INTERFACE,
            &[USB_MS_HEADER_SUBTYPE,
                0x00,
                0x01, // Revision
                0x07 + USB_MIDI_OUT_SIZE,
                0x00, ],
        );

        let read_ep = config.endpoint_bulk_out(max_packet_size);
        config.descriptor(CS_ENDPOINT, &[MS_TYPE_GENERAL, 0x01, read_ep.info().addr.into()]);
        config.descriptor(CS_INTERFACE, &[MS_MIDI_IN_JACK, 0x01, 0x01, 0x00]);

        let write_ep = config.endpoint_bulk_in(max_packet_size);
        config.descriptor(CS_ENDPOINT, &[MS_TYPE_GENERAL, 0x01, 0x02]);
        config.descriptor(CS_INTERFACE, &[MS_MIDI_OUT_JACK, 0x01, write_ep.info().addr.into(), 0x01, 0x00, 0x00, 0x00]);

        let control = state.control.write(Control {
            shared: &state.shared,
            data_if: config.interface_number(),
        });
        mem::drop(func);
        builder.handler(control);

        Self {
            // comm_ep,
            data_if,
            read_ep,
            write_ep,
            max_packet_size: max_packet_size as usize,
        }
    }

    /// Split the class into a sender and receiver.
    ///
    /// This allows concurrently sending and receiving packets from separate tasks.
    pub fn split(self) -> (Sender<'d, D>, Receiver<'d, D>) {
        (
            Sender {
                write_ep: self.write_ep,
                seq: 0,
                max_packet_size: self.max_packet_size,
            },
            Receiver {
                data_if: self.data_if,
                read_ep: self.read_ep,
            },
        )
    }
}

/// Internal state for the CDC-NCM class.
pub struct State<'a> {
    control: MaybeUninit<Control<'a>>,
    shared: ControlShared,
}

impl<'a> State<'a> {
    /// Create a new `State`.
    pub fn new() -> Self {
        Self {
            control: MaybeUninit::uninit(),
            shared: Default::default(),
        }
    }
}

/// Shared data between Control and MIDI Class
struct ControlShared {}

impl Default for ControlShared {
    fn default() -> Self {
        ControlShared {}
    }
}

struct Control<'a> {
    shared: &'a ControlShared,
    data_if: InterfaceNumber,
}

impl<'d> Handler for Control<'d> {}

/// MIDI class packet sender.
///
/// You can obtain a `Sender` with [`MidiClass::split`]
pub struct Sender<'d, D: Driver<'d>> {
    write_ep: D::EndpointIn,
    seq: u16,
    max_packet_size: usize,
}

impl<'d, D: Driver<'d>> Sender<'d, D> {
    /// Write a packet.
    ///
    /// This waits until the packet is successfully stored in the endpoint buffers.
    pub async fn write_packet(&mut self, data: &[u8]) -> Result<(), EndpointError> {
        for chunk in data.chunks(self.max_packet_size) {
            self.write_ep.write(&chunk).await?;
        }
        Ok(())
    }
}

/// MIDI class packet receiver.
///
/// You can obtain a `Receiver` with [`CdcNcmClass::split`]
pub struct Receiver<'d, D: Driver<'d>> {
    data_if: InterfaceNumber,
    read_ep: D::EndpointOut,
}

const HOST_BUFFER_SIZE: usize = 64;

impl<'d, D: Driver<'d>> Receiver<'d, D> {
    /// Write a network packet.
    ///
    /// This waits until a packet is successfully received from the endpoint buffers.
    pub async fn read_packet(&mut self, buf: &mut [Packet]) -> Result<usize, EndpointError> {
        // read NTB
        let mut ntb = [0u8; HOST_BUFFER_SIZE];
        let mut pos = 0;
        loop {
            let n = self.read_ep.read(&mut ntb[pos..]).await?;
            pos += n;
            if n < self.read_ep.info().max_packet_size as usize || pos == HOST_BUFFER_SIZE {
                break;
            }
        }
        // TODO make packets
        return Ok(pos);
    }
}


// impl midi::Transmit for UsbMidi {
//     fn transmit(&mut self, packets: PacketList) -> Result<(), MidiError> {
//         for packet in packets.iter() {
//             self.midi_class.tx_push(packet.bytes());
//         }
//         self.midi_class.tx_flush();
//         Ok(())
//     }
// }
//
// impl midi::Receive for UsbMidi {
//     fn receive(&mut self) -> Result<Option<Packet>, MidiError> {
//         if let Some(bytes) = self.midi_class.receive() {
//             return Ok(Some(Packet::from_raw(bytes)));
//         }
//         Ok(None)
//     }
// }


// impl<B: UsbBus> UsbClass<B> for MidiClass<'_, B> {
//     /// Callback after USB flush (send) completed
//     /// Check for packets that were enqueued while devices was busy (UsbErr::WouldBlock)
//     /// If any packets are pending re-flush queue immediately
//     /// This callback may chain-trigger under high output load (big sysex, etc.) - this is good
//     // fn endpoint_in_complete(&mut self, addr: EndpointAddress) {
//     //     if addr == self.bulk_in.address() && self.tx_len > 0 {
//     //         // send pending bytes in tx_buf
//     //         self.tx_flush();
//     //     }
//     // }
//
//     /// Magic copied from https://github.com/btrepp/rust-midi-stomp (thanks)
//     /// For details refer to USB MIDI spec 1.0 https://www.usb.org/sites/default/files/midi10.pdf
//     fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<(), usb_device::UsbError> {
//         writer.interface(
//             self.audio_subclass,
//             USB_AUDIO_CLASS,
//             USB_AUDIO_CONTROL_SUBCLASS,
//             0x00, // no protocol
//         )?;
//
//         writer.write(CS_INTERFACE, &[
//             USB_HEADER_SUBTYPE,
//             0x00,
//             0x01, // Revision
//             0x09,
//             0x00, // SIZE of class specific descriptions
//             0x01, // Number of streaming interfaces
//             0x01, // MIDI Streaming interface 1 belongs to this AC interface
//         ])?;
//
//         // Streaming Standard
//         writer.interface(
//             self.midi_subclass,
//             USB_AUDIO_CLASS,
//             USB_MIDI_STREAMING_SUBCLASS,
//             0,
//         )?;
//
//         // Streaming Extras
//         writer.write(CS_INTERFACE, &[
//             USB_MS_HEADER_SUBTYPE,
//             0x00,
//             0x01, // Revision
//             0x07 + USB_MIDI_OUT_SIZE,
//             0x00,
//         ])?;
//
//         // Jacks
//         writer.write(CS_INTERFACE, &[USB_MIDI_IN_JACK_SUBTYPE, USB_JACK_EMBEDDED, 0x01, 0x00])?;
//
//         writer.write(CS_INTERFACE, &[
//             USB_MIDI_OUT_JACK_SUBTYPE,
//             USB_JACK_EMBEDDED,
//             0x01,
//             0x01,
//             0x01,
//             0x01,
//             0x00,
//         ])?;
//
//         writer.endpoint(&self.bulk_out)?;
//         writer.write(USB_CS_ENDPOINT, &[USB_MS_GENERAL, 0x01, 0x01])?;
//
//         writer.endpoint(&self.bulk_in)?;
//         writer.write(USB_CS_ENDPOINT, &[USB_MS_GENERAL, 0x01, 0x01])?;
//         Ok(())
//     }
// }
