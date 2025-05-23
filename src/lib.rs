#![no_std]

use core::fmt;

use embassy_time::Timer;
use embedded_hal::i2c::I2c;

// TCA9534 registers
const TCA9534_REG_OUTPUT: u8 = 0x01;
const TCA9534_REG_POLARITY: u8 = 0x02;
const TCA9534_REG_CONFIG: u8 = 0x03;

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

    pub async fn begin(&mut self) -> Result<(), E> {
        // Initialize TCA9534 I/O expander
        // Configure all pins as outputs
        self.i2c.write(self.address, &[TCA9534_REG_CONFIG, 0x00])?;
        Timer::after_millis(10).await;

        // Set polarity to normal
        self.i2c
            .write(self.address, &[TCA9534_REG_POLARITY, 0x00])?;
        Timer::after_millis(10).await;

        // Set all outputs low initially
        self.i2c.write(self.address, &[TCA9534_REG_OUTPUT, 0x00])?;
        Timer::after_millis(10).await;

        // Initialize LCD
        self.initialize_lcd().await?;

        Ok(())
    }

    async fn initialize_lcd(&mut self) -> Result<(), E> {
        // See HD44780U datasheet "Initializing by Instruction" Figure 24 (4-Bit Interface)
        self.output.rs = 0;
        self.output.rw = 0;

        // Wait for more than 15ms after VCC rises to 4.5V
        Timer::after_millis(50).await;

        // First attempt - 8-bit mode
        self.lcd_write(0x30, true).await?;
        Timer::after_micros(4200).await;

        // Second attempt - 8-bit mode
        self.lcd_write(0x30, true).await?;
        Timer::after_micros(150).await;

        // Third attempt - 8-bit mode
        self.lcd_write(0x30, true).await?;
        Timer::after_micros(37).await;

        // Set to 4-bit mode
        self.lcd_write(0x20, true).await?;
        Timer::after_micros(37).await;

        // Function set: 4-bit mode, 2 lines, 5x8 font
        self.lcd_write(0x28, false).await?;
        Timer::after_micros(37).await;

        self.display().await?;

        self.clear().await?;

        self.left_to_right().await?;

        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0x01, false).await?;
        Timer::after_micros(1600).await;

        Ok(())
    }

    pub async fn home(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0x02, false).await?;
        Timer::after_micros(1600).await;

        Ok(())
    }

    pub async fn display(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1 << 2;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_display(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !(1 << 2);

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn cursor(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1 << 1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_cursor(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !(1 << 1);

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn blink(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state |= 1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_blink(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.display_state &= !1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn left_to_right(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state |= 1 << 1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn right_to_left(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state &= !(1 << 1);

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state |= 1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.entry_state &= !1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn scroll_display_left(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0x18, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn scroll_display_right(&mut self) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        self.lcd_write(0x1C, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub fn backlight(&mut self) -> Result<(), E> {
        self.output.led = 1;
        self.i2c_write(0x00 | (self.output.led << 3))?;
        Ok(())
    }

    pub fn no_backlight(&mut self) -> Result<(), E> {
        self.output.led = 0;
        self.i2c_write(0x00 | (self.output.led << 3))?;
        Ok(())
    }

    pub async fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        let new_address = if row == 0 { 0x00 } else { 0x40 } + col;

        self.lcd_write(0x80 | new_address, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn create_char(&mut self, location: u8, charmap: &[u8; 8]) -> Result<(), E> {
        self.output.rs = 0;
        self.output.rw = 0;

        let location = location % 8;

        self.lcd_write(0x40 | (location << 3), false).await?;
        Timer::after_micros(37).await;

        for &byte in charmap.iter() {
            self.write_byte(byte).await?;
        }

        // Set the address pointer back to the DDRAM
        self.set_cursor(0, 0).await?;
        Ok(())
    }

    pub async fn write_byte(&mut self, byte: u8) -> Result<(), E> {
        self.output.rs = 1;
        self.output.rw = 0;

        self.lcd_write(byte, false).await?;
        Timer::after_micros(41).await;

        Ok(())
    }

    pub async fn write_str(&mut self, s: &str) -> Result<(), E> {
        for byte in s.bytes() {
            self.write_byte(byte).await?;
        }
        Ok(())
    }

    async fn lcd_write(&mut self, output: u8, initialization: bool) -> Result<(), E> {
        self.output.data = output;

        // Send high nibble
        self.output.e = 1;
        self.i2c_write(self.output.get_high_data())?;
        Timer::after_micros(1).await;

        self.output.e = 0;
        self.i2c_write(self.output.get_high_data())?;

        // During initialization we only send half a byte
        if !initialization {
            Timer::after_micros(37).await;

            // Send low nibble
            self.output.e = 1;
            self.i2c_write(self.output.get_low_data())?;
            Timer::after_micros(1).await;

            self.output.e = 0;
            self.i2c_write(self.output.get_low_data())?;
        }

        Ok(())
    }

    fn i2c_write(&mut self, output: u8) -> Result<(), E> {
        self.i2c.write(self.address, &[TCA9534_REG_OUTPUT, output])
    }
}

impl<I2C, E> fmt::Write for LcdI2c<I2C>
where
    I2C: I2c<Error = E>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.output.rs = 1;
            self.output.rw = 0;
            self.output.data = byte;

            // High nibble
            self.output.e = 1;
            if self.i2c_write(self.output.get_high_data()).is_err() {
                return Err(fmt::Error);
            }

            self.output.e = 0;
            if self.i2c_write(self.output.get_high_data()).is_err() {
                return Err(fmt::Error);
            }

            // Low nibble
            self.output.e = 1;
            if self.i2c_write(self.output.get_low_data()).is_err() {
                return Err(fmt::Error);
            }

            self.output.e = 0;
            if self.i2c_write(self.output.get_low_data()).is_err() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}
