#![no_std]
#![no_main]

#![feature(slice_patterns)]
#![feature(alloc)]
#![feature(asm)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

mod boot;
mod proto;
mod ucs2;
mod gop;
mod pointer;

use uefi::{Handle, Status};
use uefi::table;

#[no_mangle]
pub extern "C" fn uefi_start(_handle: Handle, st: &'static table::SystemTable) -> Status {
    uefi_services::init(st);

    let stdout = st.stdout();
    let bt = st.boot;

    macro_rules! timeout {
        ($msg:expr, $seconds:expr) => {
            for i in 0..$seconds {
                let (_, row) = stdout.get_cursor_position();
                info!($msg, $seconds - i);
                stdout.set_cursor_position(0, row).unwrap();

                bt.stall(1_000_000);
            }

            info!($msg, 0);
        };
    }

    // Reset the console.
    {
        stdout.reset(false).expect("Failed to reset stdout");
    }

    // Switch to the maximum supported graphics mode.
    {
        let best_mode = stdout.modes().last().unwrap();
        stdout.set_mode(best_mode).expect("Failed to change graphics mode");
    }

    // Set a new color, and paint the background with it.
    {
        use ::uefi::proto::console::text::Color;
        stdout.set_color(Color::White, Color::Blue).expect("Failed to change console color");
        stdout.clear().expect("Failed to clear screen");
    }

    // Move the cursor.
    {
        stdout.enable_cursor(true).expect("Failed to enable cursor");
        stdout.set_cursor_position(24, 0).expect("Failed to move cursor");
        stdout.enable_cursor(false).expect("Failed to enable cursor");

        // This will make this `info!` line be (somewhat) centered.
        info!("# uefi-rs test runner");
    }

    {
        let revision = st.uefi_revision();
        let (major, minor) = (revision.major(), revision.minor());

        info!("UEFI {}.{}.{}", major, minor / 10, minor % 10);
    }

    info!("");

    // Print all modes.
    for (index, mode) in stdout.modes().enumerate() {
        info!("Graphics mode #{}: {} rows by {} columns", index, mode.rows(), mode.columns());
    }

    info!("");

    {
        info!("Memory Allocation Test");

        let mut values = vec![-5, 16, 23, 4, 0];

        values.sort();

        info!("Sorted vector: {:?}", values);
    }

    info!("");

    match boot::boot_services_test(bt) {
        Ok(_) => info!("Boot services test passed."),
        Err(status) => error!("Boot services test failed with status {:?}", status),
    }

    match proto::protocol_test(bt) {
        Ok(_) => info!("Protocol test passed."),
        Err(status) => error!("Protocol test failed with status {:?}", status),
    }

    match ucs2::ucs2_encoding_test() {
        Ok(_) => info!("UCS-2 encoding test passed."),
        Err(status) => error!("UCS-2 encoding test failed with status {:?}", status),
    }

    pointer::test(bt);

    info!("");

    timeout!("Testing UEFI graphics in {} second(s)...", 3);
    stdout.reset(false).unwrap();

    // Draw some graphics.
    match gop::test_graphics_output(bt) {
        Ok(_) => info!("Graphics Output Protocol test passed."),
        Err(status) => error!("Graphics test failed: {:?}", status),
    }

    // Get our text output back.
    stdout.reset(false).unwrap();

    info!("");

    timeout!("Testing complete, shutting down in {} second(s)...", 3);

    let rt = st.runtime;
    rt.reset(table::runtime::ResetType::Shutdown, Status::Success, None);
}
