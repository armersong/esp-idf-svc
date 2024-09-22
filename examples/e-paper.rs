use esp_idf_hal::gpio::{Gpio10, Gpio11, Gpio15, Gpio18, Gpio19, Gpio20, Input, Output};
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config::DriverConfig, Dma, SpiDriver};
use esp_idf_svc::log::EspLogger;
use std::thread::sleep;
use std::time::Duration;
// use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::spi::config::Config;
use esp_idf_svc::hal::spi::SpiDeviceDriver;
use esp_idf_svc::hal::spi::SpiDriverConfig;

use log::info;

const WIDTH: u16 = 212;
const HEIGHT: u16 = 104;
const EPD_2IN13_FULL: u8 = 0;
const EPD_2IN13_PART: u8 = 1;

const WHITE: u8 = 0xff;
const BLACK: u8 = 0x0;

const EPD_2IN13_LUT_FULL_UPDATE: &[u8] = &[
    0x22, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x1E, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const EPD_2IN13_LUT_PARTIAL_UPDATE: &[u8] = &[
    0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0F, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    return manual();
}

struct SpiContext<'d> {
    busy: PinDriver<'d, Gpio20, Input>,
    rst: PinDriver<'d, Gpio19, Output>,
    dc: PinDriver<'d, Gpio18, Output>,
    sclk: PinDriver<'d, Gpio11, Output>,
    sdo: PinDriver<'d, Gpio10, Output>,
    cs: PinDriver<'d, Gpio15, Output>,
}
impl<'d> SpiContext<'d> {
    pub fn config(&mut self) -> anyhow::Result<()> {
        println!("config");
        self.cs.set_high()?;
        self.sclk.set_low()?;
        Ok(())
    }
    pub fn reset(&mut self) -> anyhow::Result<()> {
        println!("reset");
        self.rst.set_high()?;
        sleep(Duration::from_millis(200));
        self.rst.set_low()?;
        sleep(Duration::from_millis(200));
        self.rst.set_high()?;
        sleep(Duration::from_millis(200));
        Ok(())
    }
    pub fn write_byte(&mut self, data: u8) -> anyhow::Result<()> {
        // println!("write_byte");
        self.cs.set_low()?;
        let mut d = data;
        for i in 0..8 {
            if data & 0x80 == 0 {
                self.sdo.set_low()?;
            } else {
                self.sdo.set_high()?;
            }
            d <<= 1;
            self.sclk.set_high()?;
            self.sclk.set_low()?;
        }
        self.cs.set_high()?;
        Ok(())
    }
    pub fn write_bytes(&mut self, data: &[u8]) -> anyhow::Result<()> {
        for v in data {
            self.write_byte(*v)?;
        }
        Ok(())
    }
    pub fn send_cmd(&mut self, data: u8) -> anyhow::Result<()> {
        self.send_data_internal(data, true)
    }
    pub fn send_data(&mut self, data: u8) -> anyhow::Result<()> {
        self.send_data_internal(data, false)
    }
    fn send_data_internal(&mut self, data: u8, is_cmd: bool) -> anyhow::Result<()> {
        if is_cmd {
            self.dc.set_low()?;
        } else {
            self.dc.set_high()?;
        }
        self.dc.set_low()?;
        self.cs.set_low()?;
        self.write_byte(data)?;
        self.cs.set_high()?;
        Ok(())
    }
    pub fn init(&mut self, mode: u8) -> anyhow::Result<()> {
        self.config()?;

        self.reset()?;

        println!("DRIVER_OUTPUT_CONTROL");
        self.send_cmd(0x1)?;
        self.send_data((HEIGHT - 1) as u8 & 0xFF)?;
        self.send_data(((HEIGHT - 1) >> 8) as u8 & 0xFF)?;
        self.send_data(0x00)?; // GD = 0; SM = 0; TB = 0;

        println!("BOOSTER_SOFT_START_CONTROL");
        self.send_cmd(0x0c)?;
        self.send_data(0xd7)?;
        self.send_data(0xd6)?;
        self.send_data(0x9d)?;

        println!("WRITE_VCOM_REGISTER");
        self.send_cmd(0x2C)?;
        self.send_data(0xA8)?; //VCOM 7C

        println!("SET_DUMMY_LINE_PERIOD");
        self.send_cmd(0x3A)?;
        self.send_data(0x1A)?; //4 dummy lines per gate

        println!("SET_GATE_TIME");
        self.send_cmd(0x3B)?;
        self.send_data(0x08)?; // 2us per line

        println!("BORDER_WAVEFORM_CONTROL");
        self.send_cmd(0x3C)?;
        self.send_data(0x63)?;

        println!("DATA_ENTRY_MODE_SETTING");
        self.send_cmd(0x11)?;
        self.send_data(0x03)?; // X increment; Y increment

        //set the look-up table register
        self.send_cmd(0x32)?;
        if mode == EPD_2IN13_FULL {
            for i in 0..EPD_2IN13_LUT_FULL_UPDATE.len() {
                self.send_data(EPD_2IN13_LUT_FULL_UPDATE[i])?;
            }
        } else if mode == EPD_2IN13_PART {
            for i in 0..EPD_2IN13_LUT_PARTIAL_UPDATE.len() {
                self.send_data(EPD_2IN13_LUT_PARTIAL_UPDATE[i])?;
            }
        } else {
            println!("invalid mode {}", mode);
        }

        Ok(())
    }

    pub fn wait_ready(&mut self) {
        Duration::from_millis(1_000);
        // while self.busy.is_high() {
        //     sleep(Duration::from_millis(100));
        // }
    }

    pub fn turnon_display(&mut self) -> anyhow::Result<()> {
        println!("DISPLAY_UPDATE_CONTROL_2");
        self.send_cmd(0x22)?;
        self.send_data(0xC4)?;
        self.send_cmd(0x20)?; //MASTER_ACTIVATION
        self.send_data(0xFF)?; //TERMINATE_FRAME_READ_WRITE
        self.wait_ready();
        Ok(())
    }

    pub fn set_window(
        &mut self,
        x_start: u16,
        y_start: u16,
        x_end: u16,
        y_end: u16,
    ) -> anyhow::Result<()> {
        println!("set window {}x{} {}x{}", x_start, y_start, x_end, y_end);
        self.send_cmd(0x44)?;
        /* x point must be the multiple of 8 or the last 3 bits will be ignored */
        self.send_data((x_start >> 3) as u8 & 0xFF)?;
        self.send_data((x_end >> 3) as u8 & 0xFF)?;

        self.send_cmd(0x45)?;
        self.send_data(y_start as u8 & 0xFF)?;
        self.send_data((y_start >> 8) as u8 & 0xFF)?;
        self.send_data(y_end as u8 & 0xFF)?;
        self.send_data((y_end >> 8) as u8 & 0xFF)?;
        Ok(())
    }
    pub fn set_cursor(&mut self, x: u16, y: u16) -> anyhow::Result<()> {
        // println!("set cursor {}x{}", x, y);
        self.send_cmd(0x4E)?;
        /* x point must be the multiple of 8 or the last 3 bits will be ignored */
        self.send_data((x >> 3) as u8 & 0xFF)?;

        self.send_cmd(0x4F)?;
        self.send_data(y as u8 & 0xFF)?;
        self.send_data((y >> 8) as u8 & 0xFF)?;
        Ok(())
    }
    pub fn sleep(&mut self) -> anyhow::Result<()> {
        println!("sleep mode");
        self.send_cmd(0x10)?; //DEEP_SLEEP_MODE
        self.send_data(0x1)?;
        Ok(())
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        println!("clear display");
        let width = if WIDTH % 8 == 0 {
            WIDTH / 8
        } else {
            WIDTH / 8 + 1
        };
        let height = HEIGHT;
        self.set_window(0, 0, WIDTH, HEIGHT)?;
        for j in 0..height {
            self.set_cursor(0, j)?;
            self.send_cmd(0x24)?;
            for i in 0..width {
                self.send_data(0xff)?;
            }
        }
        self.turnon_display()?;
        Ok(())
    }
    pub fn display(&mut self, data: &[u8]) -> anyhow::Result<()> {
        println!("show image len {}", data.len());
        let width = if WIDTH % 8 == 0 {
            WIDTH / 8
        } else {
            WIDTH / 8 + 1
        };
        let height = HEIGHT;
        self.set_window(0, 0, WIDTH, HEIGHT)?;
        for j in 0..height {
            self.set_cursor(0, j)?;
            self.send_cmd(0x24)?;
            for i in 0..width {
                let index = (i + j * width) as usize;
                if index >= data.len() {
                    continue;
                }
                self.send_data(data[index as usize])?;
            }
        }
        self.turnon_display()?;
        Ok(())
    }
}

fn manual() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let busy = PinDriver::input(peripherals.pins.gpio20)?;
    let rst = PinDriver::output(peripherals.pins.gpio19)?;
    let dc = PinDriver::output(peripherals.pins.gpio18)?;
    let sclk = PinDriver::output(peripherals.pins.gpio11)?;
    // let sdi = peripherals.pins.gpio9;
    let sdo = PinDriver::output(peripherals.pins.gpio10)?;
    let cs = PinDriver::output(peripherals.pins.gpio15)?;

    let mut ctx = SpiContext {
        busy,
        rst,
        dc,
        sclk,
        sdo,
        cs,
    };

    println!("init display");
    ctx.init(EPD_2IN13_FULL)?;
    println!("clear display");
    ctx.clear()?;
    sleep(Duration::from_millis(500));

    let mut color = WHITE;
    let mut data = [0u8; (WIDTH*HEIGHT) as usize/10];
    println!("start loop display");
    loop {
        color = if color == WHITE {
            BLACK
        } else {
            WHITE
        };
        data.fill(color);

        println!("show color 0x{:x}", color);
        ctx.display(&data)?;
        // we are using thread::sleep here to make sure the watchdog isn't triggered
        sleep(Duration::from_millis(2000));
    }
    Ok(())
}

fn spi_device() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let spi = peripherals.spi2;

    let busy = PinDriver::input(peripherals.pins.gpio20)?;
    let rst = PinDriver::output(peripherals.pins.gpio19)?;
    let dc = PinDriver::output(peripherals.pins.gpio18)?;
    let sclk = peripherals.pins.gpio11;
    let sdi = peripherals.pins.gpio9;
    let sdo = peripherals.pins.gpio10;
    let cs = peripherals.pins.gpio15;

    let cfg = Config::new();

    let mut device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sdo,
        Some(sdi),
        Some(cs),
        &SpiDriverConfig::new(),
        &cfg,
    )?;
    let mut read = [0u8; 4];
    let write = [0u8; 100];

    loop {
        // we are using thread::sleep here to make sure the watchdog isn't triggered
        device.transfer(&mut read, &write)?;
        println!("Device 1: Wrote {write:x?}, read {read:x?}");
        std::thread::sleep(Duration::from_millis(2000));
    }
    Ok(())
}
