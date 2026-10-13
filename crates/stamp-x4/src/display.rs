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

/// Raw diagnostic response, not a confirmed controller identity.
#[derive(Debug)]
pub struct DisplayProbe {
    pub status: u8,
    pub version: [u8; 5],
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
            .with_sio0(data);
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

    /// Restart the display controller before reading diagnostic registers.
    /// Timing follows the pinned FreeInk probe's longer identification reset.
    pub fn reset_for_probe(&mut self) {
        let delay = Delay::new();
        self.display_select.set_high();
        self.sd_select.set_high();

        // Establish the released level, then hold the controller in reset.
        self.reset.set_high();
        delay.delay_millis(2);
        self.reset.set_low();
        delay.delay_millis(50);

        // Release reset and allow startup before attempting register reads.
        // Busy polarity is not known yet, so this uses a fixed settling delay.
        self.reset.set_high();
        delay.delay_millis(30);
    }

    /// Read UC81xx diagnostic registers without requesting a screen refresh.
    /// A successful SPI transfer does not prove that a controller answered.
    pub fn probe(&mut self) -> Result<DisplayProbe, esp_hal::spi::Error> {
        self.reset_for_probe();
        let mut status = [0; 1];
        let mut version = [0; 5];
        self.read_command(0x71, &mut status)?;
        self.read_command(0x70, &mut version)?;
        Ok(DisplayProbe {
            status: status[0],
            version,
        })
    }

    pub fn send_command(&mut self, command: u8) -> Result<(), esp_hal::spi::Error> {
        self.command_data.set_low();
        self.display_select.set_low();
        let res = self.spi.write(&[command]);
        self.display_select.set_high();
        res
    }

    /// This is a diagnostic debug handler.
    ///
    /// We send a command to the display controller.
    /// After transmitting the command, we switch the `command_data` pin to `data` mode and then
    /// read  data over the duplex channel. This data flows from controller back
    /// to the chip/hal that is read into `response`.
    ///
    /// Note: it is important that `display_select` is "on" while we receive bytes
    /// from the controller.
    pub fn read_command(
        &mut self,
        command: u8,
        response: &mut [u8],
    ) -> Result<(), esp_hal::spi::Error> {
        self.command_data.set_low();
        self.display_select.set_low();
        let result = match self.spi.write(&[command]) {
            Ok(()) => {
                // Switch to data phase so that the display controller can send
                // data over the spi.
                self.command_data.set_high();
                self.spi.half_duplex_read(
                    esp_hal::spi::master::DataMode::Single,
                    esp_hal::spi::master::Command::None,
                    esp_hal::spi::master::Address::None,
                    0,
                    response,
                )
            }
            Err(err) => Err(err),
        };
        self.display_select.set_high();
        result
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
