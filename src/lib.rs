#![no_std]

use core::fmt;

use embassy_time::Timer;
use embedded_hal::i2c::I2c;

pub struct OutputState {
    rs: u8,
    rw: u8,
    e: u8,
    led: u8,
    data: u8,
}

impl OutputState {
    fn new() -> Self {
        Self {
            rs: 0,
            rw: 0,
            e: 0,
            led: 0,
            data: 0,
        }
    }

    fn get_low_data(&self) -> u8 {
        let mut buffer = self.rs;

        buffer |= self.rw << 1;
        buffer |= self.e << 2;
        buffer |= self.led << 3;
        buffer |= (self.data & 0x0F) << 4;
        buffer
    }

    fn get_high_data(&self) -> u8 {
        let mut buffer = self.rs;

        buffer |= self.rw << 1;
        buffer |= self.e << 2;
        buffer |= self.led << 3;
        buffer |= self.data & 0xF0;
        buffer
    }
}

pub struct LcdI2c<I2C> {
    i2c: I2C,
    address: u8,
    output: OutputState,
    display_state: u8,
    entry_state: u8,
}

impl<I2C, E> LcdI2c<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self {
            i2c,
            address,
            output: OutputState::new(),
            display_state: 0x00,
            entry_state: 0x00,
        }
    }

    pub async fn display(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1 << 2;

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0b00000001, false).await?;
        Timer::after_millis(1600).await;

        Ok(())
    }

    pub async fn home(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0b00000010, false).await?;
        Timer::after_millis(1600).await;

        Ok(())
    }

    pub async fn left_to_right(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state |= 1 << 1;

        self.lcd_write(0b00000100 | self.entry_state, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Set text direction to right-to-left
    pub async fn right_to_left(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state &= !(1 << 1);

        self.lcd_write(0b00000100 | self.entry_state, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    pub fn backlight(&mut self) -> Result<(), E> {
        self.output.led = 1;
        self.i2c_write(0b00000000 | (self.output.led << 3))?;
        Ok(())
    }

    /// Turn off the backlight
    pub fn no_backlight(&mut self) -> Result<(), E> {
        self.output.led = 0;
        self.i2c_write(0b00000000 | (self.output.led << 3))?;
        Ok(())
    }

    pub async fn autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state |= 1;

        self.lcd_write(0b00000100 | self.entry_state, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Disable autoscroll
    pub async fn no_autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state &= !1;

        self.lcd_write(0b00000100 | self.entry_state, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    pub async fn no_display(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !(1 << 2);

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Show cursor
    pub async fn cursor(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1 << 1;

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Hide cursor
    pub async fn no_cursor(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !(1 << 1);

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Enable cursor blinking
    pub async fn blink(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1;

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Disable cursor blinking
    pub async fn no_blink(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !1;

        self.lcd_write(0b00001000 | self.display_state, false)
            .await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Scroll display to the left
    pub async fn scroll_display_left(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0b00011000, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    /// Scroll display to the right
    pub async fn scroll_display_right(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0b00011100, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    pub async fn write_byte(&mut self, byte: u8) -> Result<(), E> {
        self.output.rs = 1;
        self.output.rw = 0;

        self.lcd_write(byte, false).await?;
        Timer::after_millis(41).await;

        Ok(())
    }

    /// Write a string to the LCD
    pub async fn write_str(&mut self, s: &str) -> Result<(), E> {
        for byte in s.bytes() {
            self.write_byte(byte).await?;
        }
        Ok(())
    }

    /// Create a custom character
    pub async fn create_char(&mut self, location: u8, charmap: &[u8; 8]) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        let location = location % 8;

        self.lcd_write(0b01000000 | (location << 3), false).await?;
        Timer::after_millis(37).await;

        for &byte in charmap.iter() {
            self.write_byte(byte).await?;
        }

        // Set the address pointer back to the DDRAM
        self.set_cursor(0, 0).await?;
        Ok(())
    }

    /// Set cursor position
    pub async fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        let new_address = if row == 0 { 0x00 } else { 0x40 } + col;

        self.lcd_write(0b10000000 | new_address, false).await?;
        Timer::after_millis(37).await;

        Ok(())
    }

    pub async fn begin(&mut self) -> Result<(), E> {
        self.i2c.write(self.address, &[0x03, 0x00])?;
        Timer::after_millis(10).await;

        self.i2c.write(self.address, &[0x02, 0x00])?;
        Timer::after_millis(10).await;

        self.i2c.write(self.address, &[0x01, 0x00])?;
        Timer::after_millis(10).await;

        self.initialize_lcd().await?;

        Ok(())
    }

    async fn initialize_lcd(&mut self) -> Result<(), E> {
        // See HD44780U datasheet "Initializing by Instruction" Figure 24 (4-Bit Interface)
        self.output.rs = 0;
        self.output.rw = 0;

        Timer::after_millis(150).await;
        self.lcd_write(0b00110000, true).await?;

        Timer::after_millis(50).await;
        self.lcd_write(0b00110000, true).await?;

        Timer::after_millis(37).await;
        self.lcd_write(0b00110000, true).await?;

        Timer::after_millis(37).await;
        self.lcd_write(0b00100000, true).await?; // Function Set - 4 bits mode

        Timer::after_millis(37).await;
        self.lcd_write(0b00101000, false).await?; // Function Set - 4 bits(Still), 2 lines, 5x8 font

        self.no_display().await?;
        self.clear().await?;
        self.left_to_right().await?;

        Ok(())
    }

    fn i2c_write(&mut self, output: u8) -> Result<(), E> {
        self.i2c.write(self.address, &[0x01, output])
    }

    async fn lcd_write(&mut self, output: u8, initialization: bool) -> Result<(), E> {
        self.output.data = output;

        // Send high nibble
        self.output.e = 1;
        self.i2c_write(self.output.get_high_data())?;

        Timer::after_millis(1).await;
        // High part of enable should be >450 nS
        // We rely on I2C transaction taking more than 450ns

        self.output.e = 0;
        self.i2c_write(self.output.get_high_data())?;

        // During initialization we only send half a byte
        if !initialization {
            Timer::after_millis(37).await;
            // We need a delay between half byte writes

            // Send low nibble
            self.output.e = 1;
            self.i2c_write(self.output.get_low_data())?;
            // High part of enable should be >450 nS
            Timer::after_millis(1).await;

            self.output.e = 0;
            self.i2c_write(self.output.get_low_data())?;
        }

        Ok(())
    }
}

impl<I2C, E> fmt::Write for LcdI2c<I2C>
where
    I2C: I2c<Error = E>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // This implementation can't handle errors from the I2C bus
        // so we just ignore them and return Ok
        for byte in s.bytes() {
            let _ = self.output.rs = 1;
            let _ = self.output.rw = 0;
            let _ = self.lcd_write(byte, false);
            // We should delay here, but can't in this context
        }
        Ok(())
    }
}
