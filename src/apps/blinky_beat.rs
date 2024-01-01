use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use midi::{Note, note_off, note_on, Velocity, PacketList, MidiChannel, MidiError, channel};
use midi::MidiChannel::CH1;
use crate::{AppError, devices, MIDI_DIN_1_OUT};

use devices::arturia::beatstep;
use beatstep::Param::*;
use beatstep::Pad::*;
use crate::devices::arturia::beatstep::{SwitchMode};
use crate::resource::Shared;

#[derive(Debug)]
struct InnerState {
    channel: MidiChannel,
    notes: Vec<(Note, bool), 16>,
}

impl InnerState {}

static BLINKY_BEAT: Shared<InnerState> = Shared::uninit("BLINKY_BEAT");

#[embassy_executor::task]
async fn blinky() -> ! {
    let mut z = BLINKY_BEAT.lock().await;

    // force set MIDI channel (simulate manual selection with CHAN+PAD)
    // FIXME neither of these work, is setting channel possible?
    //   alt: read current MIDI channel and use it (maybe even better?)
    midi_send(beatstep::beatstep_set(CVGateChannel(CH1)).into()).await;
    midi_send(beatstep::beatstep_set(GlobalMidiChannel(CH1)).into()).await;

    midi_send(beatstep::beatstep_set(PadNote(Pad(0), z.get().unwrap().channel, Note::C1m, SwitchMode::Gate)).into()).await;

    // turn off all LED pads
    for (note, _) in &mut z.get_mut().unwrap().notes {
        midi_send(PacketList::single(note_off(CH1, *note, Velocity::MIN).unwrap().into())).await;
    }

    // chase LEDs across all pads
    loop {
        for (note, _) in &mut z.get_mut().unwrap().notes {
            midi_send(PacketList::single(note_on(CH1, *note, Velocity::MAX).unwrap().into())).await;
            Timer::after(Duration::from_millis(50)).await;
            midi_send(PacketList::single(note_off(CH1, *note, Velocity::MIN).unwrap().into())).await;
        }
    }
}

async fn midi_send(packets: PacketList) {
    let mut bs_out = MIDI_DIN_1_OUT.lock().await;
    bs_out.get_mut().unwrap().transmit(packets.into()).await.unwrap();
}

pub async fn start_app(channel: MidiChannel, notes: &[Note], spawner: Spawner) -> Result<(), AppError> {
    BLINKY_BEAT.lock().await.set(InnerState {
        channel,
        notes: notes.iter().map(|n| (*n, false)).collect(),
    }).unwrap();

    spawner.spawn(blinky())?;

    info!("BlinkyBeat Active");
    Ok(())
}
