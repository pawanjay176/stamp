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
    Spi(esp_hal::spi::Error),
    BusyTimeout,
    RefreshDidNotStart,
}

impl From<esp_hal::spi::Error> for DisplayError {
    fn from(error: esp_hal::spi::Error) -> Self {
        Self::Spi(error)
    }
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

    /// Restart the display controller before diagnostics or initialization.
    /// Timing follows the pinned FreeInk probe's longer identification reset.
    pub fn hardware_reset(&mut self) {
        let delay = Delay::new();
        self.display_select.set_high();
        self.sd_select.set_high();

        // Establish the released level, then hold the controller in reset.
        self.reset.set_high();
        delay.delay_millis(2);
        self.reset.set_low();
        delay.delay_millis(50);

        // Release reset and allow startup. A fixed delay also supports probes
        // where the controller's Busy polarity is not known yet.
        self.reset.set_high();
        delay.delay_millis(30);
    }

    /// Reset and configure the SSD1677 operating profile for the X4 panel.
    /// This does not upload pixels, refresh the panel, or identify the controller.
    pub fn init_ssd1677(&mut self) -> Result<(), DisplayError> {
        self.hardware_reset();
        self.send_command(0x12)?; // SSD1677 software reset.

        // Busy may assert after the command returns. Match the reference's
        // settling delay before checking the active-high Busy signal.
        Delay::new().delay_millis(10);
        self.wait_until_idle(esp_hal::gpio::Level::High, Duration::from_secs(1))?;

        // temperature sensor selection. E-ink behaves differently at different temperatures.
        // Sending 0x80 selects the controller’s internal sensor, whose measurement can be used when
        // selecting refresh timing.
        // TODO(pawan): wtf does it mean? simply copying from crossink reference for now
        self.send_command(0x18)?;
        self.send_data(&[0x80])?;

        // X4’s panel power-startup settings
        self.send_command(0x0c)?;
        self.send_data(&[0xae, 0xc7, 0xc3, 0xc0, 0x80])?;

        // Configure 480 rows and the panel’s scan arrangement
        self.send_command(0x01)?;
        self.send_data(&[0xdf, 0x01, 0x02])?;

        // Configure the screen border
        self.send_command(0x3c)?;
        self.send_data(&[0x80])?;

        self.set_full_ram_window()?;

        Ok(())
    }

    /// Configure the destination region and write order for a full-screen RAM upload.
    ///
    /// The X4 uses 800 columns and 480 rows. Its reference orientation writes
    /// X from 0 to 799, then steps Y from 479 down to 0. Each incoming byte
    /// represents eight horizontal pixels, so a row takes 100 bytes and a
    /// complete image plane takes 48,000 bytes.
    ///
    /// Commands 0x44/0x45 set the bounds; 0x11 sets the traversal direction.
    /// Commands 0x4E/0x4F set the starting position to (0, 479). Coordinates
    /// are sent low byte first: 799 is [0x1F, 0x03], 479 is [0xDF, 0x01].
    ///
    /// Call before writing each complete image plane to start at its origin.
    /// This configures existing controller RAM; it does not allocate or clear
    /// memory, transfer pixels, or trigger a screen refresh. SPI errors propagate.
    pub fn set_full_ram_window(&mut self) -> Result<(), DisplayError> {
        self.send_command(0x11)?;
        self.send_data(&[0x01])?;

        self.send_command(0x44)?;
        self.send_data(&[0x00, 0x00, 0x1f, 0x03])?;

        self.send_command(0x45)?;
        self.send_data(&[0xdf, 0x01, 0x00, 0x00])?;

        self.send_command(0x4e)?;
        self.send_data(&[0x00, 0x00])?;

        self.send_command(0x4f)?;
        self.send_data(&[0xdf, 0x01])?;

        Ok(())
    }

    pub fn write_test_pattern(&mut self) -> Result<(), DisplayError> {
        // Begin writing primary image memory
        self.set_full_ram_window()?;
        self.send_command(0x24)?;

        let mut row = [0u8; 100];
        for y in 0..480 {
            let white = (y / 60) % 2 == 0;
            row.fill(if white { 0xFF } else { 0x00 });
            self.send_data(&row)?;
        }

        // Write the same to the "second plane"
        self.set_full_ram_window()?;
        self.send_command(0x26)?;

        for y in 0..480 {
            let white = (y / 60) % 2 == 0;
            row.fill(if white { 0xFF } else { 0x00 });
            self.send_data(&row)?;
        }

        Ok(())
    }

    pub fn words(&mut self, data: &[u8]) -> Result<(), DisplayError> {
        // Begin writing primary image memory
        self.set_full_ram_window()?;
        self.send_command(0x24)?;
        self.send_data(data)?;

        self.set_full_ram_window()?;
        self.send_command(0x26)?;
        self.send_data(data)?;
        Ok(())
    }

    /// Display the previously uploaded image using the X4 full-refresh sequence.
    /// Call after initialization and uploading both image planes.
    ///
    /// Success means Busy was observed high and then low; it does not verify
    /// the visible image. A timeout ends our wait, not the controller's operation.
    /// Stop issuing display operations on error until the fault is investigated.
    pub fn refresh_full(&mut self) -> Result<(), DisplayError> {
        // Start from idle so an earlier operation's Busy pulse cannot be
        // mistaken for this refresh. SSD1677 Busy is active-high.
        self.wait_until_idle(esp_hal::gpio::Level::High, Duration::from_secs(30))?;

        self.send_command(0x21)?;
        self.send_data(&[0x40])?; // Bypass the second plane for full refresh.
        self.send_command(0x3C)?;
        self.send_data(&[0xC0])?; // X4 full-refresh border setting.
        self.send_command(0x22)?;
        self.send_data(&[0xF7])?; // Stored full waveform, with drive power-off at end.
        self.send_command(0x20)?; // Activate the selected update sequence.

        // Busy can assert after the SPI command has finished. Do not interpret
        // the initial low level as completion. Limits are diagnostic policy.
        let started = Instant::now();
        let delay = Delay::new();
        while !self.busy.is_high() {
            if started.elapsed() >= Duration::from_millis(100) {
                return Err(DisplayError::RefreshDidNotStart);
            }
            delay.delay_millis(1);
        }

        self.wait_until_idle(esp_hal::gpio::Level::High, Duration::from_secs(30))
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
