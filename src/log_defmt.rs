use defmt_rtt as _;
// use embassy_time::Instant;

extern crate panic_probe as _;

// embassy provided?
// defmt::timestamp!("{=u64}", {
//     Instant::now().as_millis()
// });

#[cfg(all(target_arch = "arm", target_os = "none"))]
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
