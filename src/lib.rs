#![no_std]

use core::fmt;

use embassy_time::Timer;
use embedded_hal::i2c::I2c;

// TCA9534 registers
const TCA9534_REG_OUTPUT: u8 = 0x01;
const TCA9534_REG_POLARITY: u8 = 0x02;
const TCA9534_REG_CONFIG: u8 = 0x03;

pub struct OutputState {
    rs: bool,
    rw: bool,
    e: bool,
    led: bool,
    data: u8,
}

impl OutputState {
    fn new() -> Self {
        Self {
            rs: false,
            rw: false,
            e: false,
            led: false,
            data: 0,
        }
    }

    fn get_high_data(&self) -> u8 {
        let mut buffer = 0;

        if self.rs {
            buffer |= 0x01;
        }
        if self.rw {
            buffer |= 0x02;
        }
        if self.e {
            buffer |= 0x04;
        }
        if self.led {
            buffer |= 0x08;
        }

        buffer |= self.data & 0xF0;

        buffer
    }

    fn get_low_data(&self) -> u8 {
        let mut buffer = 0;

        if self.rs {
            buffer |= 0x01;
        }
        if self.rw {
            buffer |= 0x02;
        }
        if self.e {
            buffer |= 0x04;
        }
        if self.led {
            buffer |= 0x08;
        }

        buffer |= (self.data & 0x0F) << 4;

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
        self.i2c.write(self.address, &[TCA9534_REG_CONFIG, 0x00])?;
        Timer::after_millis(10).await;

        // Set polarity to normal
        self.i2c
            .write(self.address, &[TCA9534_REG_POLARITY, 0x00])?;
        Timer::after_millis(10).await;

        // Set all outputs low
        self.i2c.write(self.address, &[TCA9534_REG_OUTPUT, 0x00])?;
        Timer::after_millis(10).await;

        self.initialize_lcd().await?;

        Ok(())
    }

    async fn initialize_lcd(&mut self) -> Result<(), E> {
        // See HD44780U datasheet "Initializing by Instruction" Figure 24 (4-Bit Interface)
        self.output.rs = false;
        self.output.rw = false;

        Timer::after_millis(50).await;

        self.lcd_write(0x30, true).await?;
        Timer::after_millis(5).await;

        self.lcd_write(0x30, true).await?;
        Timer::after_micros(150).await;

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
        self.output.rs = false;
        self.output.rw = false;

        self.lcd_write(0x01, false).await?;
        Timer::after_millis(2).await;

        Ok(())
    }

    pub async fn home(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.lcd_write(0x02, false).await?;
        Timer::after_millis(2).await;

        Ok(())
    }

    pub async fn display(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state |= 1 << 2;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_display(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state &= !(1 << 2);

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn cursor(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state |= 1 << 1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_cursor(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state &= !(1 << 1);

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn blink(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state |= 1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_blink(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.display_state &= !1;

        self.lcd_write(0x08 | self.display_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn left_to_right(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.entry_state |= 1 << 1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn right_to_left(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.entry_state &= !(1 << 1);

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.entry_state |= 1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn no_autoscroll(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.entry_state &= !1;

        self.lcd_write(0x04 | self.entry_state, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn scroll_display_left(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.lcd_write(0x18, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn scroll_display_right(&mut self) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        self.lcd_write(0x1C, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub fn backlight(&mut self) -> Result<(), E> {
        self.output.led = true;
        self.i2c_write(0x00 | (self.output.led as u8) << 3)?;
        Ok(())
    }

    pub fn no_backlight(&mut self) -> Result<(), E> {
        self.output.led = false;
        self.i2c_write(0x00 | (self.output.led as u8) << 3)?;
        Ok(())
    }

    pub async fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

        let new_address = if row == 0 { 0x00 } else { 0x40 } + col;

        self.lcd_write(0x80 | new_address, false).await?;
        Timer::after_micros(37).await;

        Ok(())
    }

    pub async fn create_char(&mut self, location: u8, charmap: &[u8; 8]) -> Result<(), E> {
        self.output.rs = false;
        self.output.rw = false;

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
        self.output.rs = true;
        self.output.rw = false;

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
        self.output.e = true;
        self.i2c_write(self.output.get_high_data())?;
        Timer::after_micros(1).await;

        self.output.e = false;
        self.i2c_write(self.output.get_high_data())?;

        // During initialization we only send half a byte
        if !initialization {
            Timer::after_micros(37).await;

            // Send low nibble
            self.output.e = true;
            self.i2c_write(self.output.get_low_data())?;
            Timer::after_micros(1).await;

            self.output.e = false;
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
            self.output.rs = true;
            self.output.rw = false;
            self.output.data = byte;

            // High nibble
            self.output.e = true;
            if self.i2c_write(self.output.get_high_data()).is_err() {
                return Err(fmt::Error);
            }

            self.output.e = false;
            if self.i2c_write(self.output.get_high_data()).is_err() {
                return Err(fmt::Error);
            }

            // Low nibble
            self.output.e = true;
            if self.i2c_write(self.output.get_low_data()).is_err() {
                return Err(fmt::Error);
            }

            self.output.e = false;
            if self.i2c_write(self.output.get_low_data()).is_err() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}
