#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(alloc_error_handler)]

extern crate embedded_midi as midi;

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate bitfield;

#[macro_use]
extern crate defmt;

use cortex_m::peripheral::SCB;
use cortex_m::peripheral::scb::FpuAccessMode;
use cortex_m::Peripherals;
use defmt::*;
use embassy_executor::Spawner;

use embassy_stm32::usart::{BufferedUart, BufferedUartRx, BufferedUartTx, Uart, UartRx, UartTx};
use embassy_stm32::{bind_interrupts, peripherals, rng, usart};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::PA1;

#[cfg(feature = "rng")]
use embassy_stm32::{
    rng::Rng,
    peripherals::{RNG},
};

use embassy_stm32::time::mhz;
use embassy_stm32::usb_otg::Driver;

use embassy_usb::{UsbDevice};

use static_cell::make_static;

use crate::port::midi_usb;
use crate::port::midi_usb::MidiClass;

use embassy_stm32::usb_otg;

use embassy_time::{Duration, Timer};
use crate::port::serial_buffered::{BufferedSerialMidiIn, BufferedSerialMidiOut};
use crate::resource::Shared;

mod resource;
mod apps;
mod devices;
mod port;
mod sysex;
mod allocator;
mod log_defmt;

pub const CPU_FREQ: u32 = 48_000_000;

#[cfg(feature = "stm32f4")]
bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
    USART2 => usart::BufferedInterruptHandler<peripherals::USART2>;
    OTG_FS => usb_otg::InterruptHandler<peripherals::USB_OTG_FS>;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

#[cfg(feature = "stm32h7")]
bind_interrupts!(struct Irqs {
    UART5 => usart::BufferedInterruptHandler<peripherals::UART5>;
    UART4 => usart::BufferedInterruptHandler<peripherals::UART4>;
    OTG_FS => usb_otg::InterruptHandler<peripherals::USB_OTG_FS>;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

// static SHARED: Shared<BufferedUartTx<'_, embassy_stm32::peripherals::UART5>> = Shared::uninit("UART5");
// SHARED.lock().await.set(uart4_tx);
// unwrap!(uart4.get_mut().unwrap().write_all("V".as_bytes()).await);

static MIDI_DIN_2_OUT: Shared<BufferedSerialMidiOut<'static, peripherals::UART5>> = Shared::uninit("MIDI_DIN_2_OUT");
static MIDI_DIN_2_IN: Shared<BufferedSerialMidiIn<'static, peripherals::UART5>> = Shared::uninit("MIDI_DIN_2_IN");

static MIDI_DIN_1_OUT: Shared<BufferedSerialMidiOut<'static, peripherals::UART4>> = Shared::uninit("MIDI_DIN_1_OUT");
static MIDI_DIN_1_IN: Shared<BufferedSerialMidiIn<'static, peripherals::UART4>> = Shared::uninit("MIDI_DIN_1_IN");

#[cfg(feature = "usb")]
static MIDI_USB_1_OUT: Shared<midi_usb::Sender<'static, Driver<'static, peripherals::USB_OTG_FS>>> = Shared::uninit("MIDI_USB_1_OUT");
#[cfg(feature = "usb")]
static MIDI_USB_1_IN: Shared<midi_usb::Receiver<'static, Driver<'static, peripherals::USB_OTG_FS>>> = Shared::uninit("MIDI_USB_1_IN");

#[cfg(feature = "rng")]
static CHAOS: Shared<rng::Rng<'static, RNG>> = Shared::uninit("CHAOS");

type UsbDriver = Driver<'static, peripherals::USB_OTG_FS>;

#[embassy_executor::task]
async fn usb_task(mut device: UsbDevice<'static, UsbDriver>) -> ! {
    device.run().await
}

#[embassy_executor::task]
async fn blink(led: &'static mut Output<'static, PA1>) -> ! {
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(100)).await;
        led.set_low();
        Timer::after(Duration::from_millis(100)).await;
    }
}

// use midi::{MidiMessage, Packet, PacketList, Velocity};
// use midi::MidiChannel::CH1;
// use midi::Note::C1;

// #[embassy_executor::task]
// async fn ping_uart5() -> ! {
//     let mut midi2_out = MIDI_DIN_2_OUT.lock().await;
//     loop {
//         let p = Packet::from(MidiMessage::NoteOn(CH1, C1, Velocity::MAX));
//         if let Err(err) = midi2_out.get_mut().unwrap().transmit(PacketList::single(p)).await {
//             error!("uh {}", err)
//         }
//         Timer::after(Duration::from_millis(500)).await;
//
//         let p = Packet::from(MidiMessage::NoteOff(CH1, C1, Velocity::MAX));
//         if let Err(err) = midi2_out.get_mut().unwrap().transmit(PacketList::single(p)).await {
//             error!("uh {}", err)
//         }
//         Timer::after(Duration::from_millis(500)).await;
//     }
// }
//
// #[embassy_executor::task]
// async fn echo_uart4() -> ! {
//     let mut midi1_out = MIDI_DIN_1_OUT.lock().await;
//     let mut midi1_in = MIDI_DIN_1_IN.lock().await;
//     loop {
//         if let Ok(packet) = midi1_in.get_mut().unwrap().receive().await {
//             if let Err(err) =  midi1_out.get_mut().unwrap().transmit(PacketList::single(packet)).await {
//                 error!("oups {}", err)
//             }
//         }
//     }
// }
//
// #[embassy_executor::task]
// async fn print_uart5() -> ! {
//     let mut midi2_in = MIDI_DIN_2_IN.lock().await;
//     loop {
//         if let Ok(packet) = midi2_in.get_mut().unwrap().receive().await {
//             let message = MidiMessage::try_from(packet).unwrap();
//             info!("{}", message);
//         }
//     }
// }

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut core_peri = cortex_m::Peripherals::take().unwrap();

    // taken from stm32h7xx-hal
    core_peri.SCB.enable_icache();
    // // See Errata Sheet 2.2.1
    // watchout for DMA
    // // core_peri.SCB.enable_dcache(&mut core_peri.CPUID);
    // core_peri.DWT.enable_cycle_counter();

    let config = embassy_stm32::Config::default();
    #[cfg(feature = "stm32f4")]
    {
        config.rcc.pll48 = true;
        config.rcc.sys_ck = Some(mhz(48));
    }
    let p = embassy_stm32::init(config);

    info!("Boot seq icache:{} dcache:{} fpu: {}", SCB::icache_enabled(), SCB::dcache_enabled(), SCB::fpu_access_mode() == FpuAccessMode::Enabled);

    // Generate random seed.
    #[cfg(feature = "rng")]
    {
        // FIXME hardfaults on devebox, blackpill
        let mut rng: rng::Rng<'_, RNG> = Rng::new(p.RNG, Irqs);
        let mut seed = [0; 8];
        let _ = rng.async_fill_bytes(&mut seed).await;
        let _seed = u64::from_le_bytes(seed);
        CHAOS.init_static(rng);
    }

    #[cfg(feature = "usb")]
    {
        // Create the driver, from the HAL.
        let ep_out_buffer = &mut make_static!([0; 256])[..];
        let mut config = usb_otg::Config::default();
        config.vbus_detection = true;
        let driver = Driver::new_fs(p.USB_OTG_FS, Irqs, p.PA12, p.PA11, ep_out_buffer, config);

        let mut usb_cfg = embassy_usb::Config::new(0xc0de, 0xcafe);
        usb_cfg.manufacturer = Some("M'Roto");
        usb_cfg.product = Some("DW-666");
        usb_cfg.serial_number = Some("666999");
        usb_cfg.max_power = 100;
        usb_cfg.max_packet_size_0 = 64;

        // Required for Windows support.
        usb_cfg.composite_with_iads = true;
        usb_cfg.device_class = 0xEF;
        usb_cfg.device_sub_class = 0x02;
        usb_cfg.device_protocol = 0x01;

        // Create embassy-usb DeviceBuilder using the driver and config.
        let mut usb_builder = embassy_usb::Builder::new(
            driver,
            usb_cfg,
            &mut make_static!([0; 256])[..],
            &mut make_static!([0; 256])[..],
            &mut make_static!([0; 256])[..],
            &mut make_static!([0; 128])[..],
        );

        let usb_midi_state = make_static!(midi_usb::State::new());
        let usb_midi = MidiClass::new(&mut usb_builder, usb_midi_state, 64);
        let usb_bus = usb_builder.build();
        let (tx, rx) = usb_midi.split();
        MIDI_USB_1_OUT.init_static(tx);
        MIDI_USB_1_IN.init_static(rx);

        unwrap!(spawner.spawn(usb_task(usb_bus)));
    }

    let mut config = usart::Config::default();
    config.baudrate = 31250;
    let tx_buf = make_static!([0u8; 32]);
    let rx_buf = make_static!([0u8; 32]);
    // let mut uart1 = Uart::new(p.UART7, p.PF6, p.PF7, Irqs, p.DMA1_CH0, NoDma, config);
    // let mut uart1 = Uart::new(p.UART7, p.PA8, p.PA15, Irqs, NoDma, NoDma, config);
    let uart5 = BufferedUart::new(p.UART5, Irqs, p.PB5, p.PB6, tx_buf, rx_buf, config).unwrap();
    let (uart5_tx, uart5_rx) = uart5.split();
    let _ = MIDI_DIN_2_OUT.lock().await.set(BufferedSerialMidiOut::new(uart5_tx));
    let _ = MIDI_DIN_2_IN.lock().await.set(BufferedSerialMidiIn::new(uart5_rx));

    let mut config = usart::Config::default();
    config.baudrate = 115200;
    let tx_buf = make_static!([0u8; 32]);
    let rx_buf = make_static!([0u8; 32]);
    let uart4 = BufferedUart::new(p.UART4, Irqs, p.PD0, p.PD1, tx_buf, rx_buf, config).unwrap();
    let (uart4_tx, uart4_rx) = uart4.split();
    let _ = MIDI_DIN_1_OUT.lock().await.set(BufferedSerialMidiOut::new(uart4_tx));
    let _ = MIDI_DIN_1_IN.lock().await.set(BufferedSerialMidiIn::new(uart4_rx));

    let led = Output::new(p.PA1, Level::High, Speed::Low);
    let led = make_static!(led);
    unwrap!(spawner.spawn(blink(led)));

    // unwrap!(spawner.spawn(ping_uart5()));
    // unwrap!(spawner.spawn(echo_uart4()));
    // unwrap!(spawner.spawn(print_uart5()));

    apps::dw6_control::start_app(spawner).await.unwrap()
}



