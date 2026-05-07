#![no_std]
#![no_main]

use bsp::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use rp_pico as bsp;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_serial::embedded_io::Write;
use usbd_serial::SerialPort;

use bsp::hal::{self, clocks::init_clocks_and_plls, pac, watchdog::Watchdog};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Zach")
            .product("Pico Serial")
            .serial_number("TEST")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    let mut number: u16 = 0;
    let mut next_write_at = 0;

    loop {
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        if usb_dev.state() == UsbDeviceState::Configured && serial.dtr() {
            let now = timer.get_counter().ticks();
            if now >= next_write_at {
                let _ = write!(serial, "{}\r\n", number);
                number = number.wrapping_add(1);
                next_write_at = now + 500_000;
            }
        }
    }
}
