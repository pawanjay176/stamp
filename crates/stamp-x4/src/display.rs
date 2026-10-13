use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, InputConfig, Output, OutputConfig},
    peripherals::{GPIO4, GPIO5, GPIO6, GPIO8, GPIO10, GPIO12, GPIO21, SPI2},
    spi::master::{Config, Spi},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub enum DisplayError {
    BusyTimeout,
}

/// A container that is used for interacting with the display controller from
/// the chip.
pub struct DisplayDriver<'a> {
    /// Serial peripheral interface. Transfers data between the device and display/sd card.
    spi: Spi<'a, Blocking>,
    /// Setting this pin (GPIO21) decides if we are sending to the display.
    /// Low corresponds to selecting and high corresponds to unselecting.
    display_select: Output<'a>,
    /// Setting this pin (GPIO4) decides if we are sending command or data over the spi.
    /// Low corresponds to command and High corresponds to data.
    command_data: Output<'a>,
    /// Setting this pin (GPIO5) decides if the display is to be reset.
    reset: Output<'a>,
    /// Reading of the busy gpio pin (GPIO6). Helps us decide if the display is busy doing
    /// stuff.
    busy: Input<'a>,
    /// Setting this pin (GPIO12) decides if we have selected the sd card
    /// Low corresponds to selecting and high corresponds to unselecting.
    sd_select: Output<'a>,
}

impl<'a> DisplayDriver<'a> {
    pub fn new(
        spi_peripheral: SPI2<'a>,
        clock: GPIO8<'a>,
        data: GPIO10<'a>,
        display: GPIO21<'a>,
        command_data: GPIO4<'a>,
        reset: GPIO5<'a>,
        busy: GPIO6<'a>,
        sd_select: GPIO12<'a>,
    ) -> Result<Self, &'static str> {
        let display_select =
            Output::new(display, esp_hal::gpio::Level::High, OutputConfig::default());
        let sd_select = Output::new(
            sd_select,
            esp_hal::gpio::Level::High,
            OutputConfig::default(),
        );
        let spi = Spi::new(spi_peripheral, Config::default())
            .map_err(|_| "failed to apply config")?
            .with_sck(clock)
            .with_mosi(data);
        let command_data = Output::new(
            command_data,
            esp_hal::gpio::Level::Low,
            OutputConfig::default(),
        );
        let reset = Output::new(reset, esp_hal::gpio::Level::High, OutputConfig::default());
        let busy = Input::new(busy, InputConfig::default());
        Ok(Self {
            spi,
            display_select,
            command_data,
            reset,
            busy,
            sd_select,
        })
    }

    pub fn send_command(&mut self, command: u8) -> Result<(), esp_hal::spi::Error> {
        self.command_data.set_low();
        self.display_select.set_low();
        let res = self.spi.write(&[command]);
        self.display_select.set_high();
        res
    }

    pub fn send_data(&mut self, data: &[u8]) -> Result<(), esp_hal::spi::Error> {
        self.command_data.set_high();
        self.display_select.set_low();
        let res = self.spi.write(data);
        self.display_select.set_high();
        res
    }

    pub fn wait_until_idle(
        &self,
        busy_level: esp_hal::gpio::Level,
        timeout: esp_hal::time::Duration,
    ) -> Result<(), DisplayError> {
        let now = Instant::now();
        let delay = Delay::new();
        loop {
            if busy_level != self.busy.level() {
                return Ok(());
            }
            if now.elapsed() >= timeout {
                return Err(DisplayError::BusyTimeout);
            }
            delay.delay(Duration::from_millis(1));
        }
    }
}
