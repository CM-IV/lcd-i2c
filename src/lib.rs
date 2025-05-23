#![no_std]

use core::fmt;

use embassy_time::Timer;
use embedded_hal::i2c::I2c;

// LCD commands
const LCD_CLEARDISPLAY: u8 = 0x01;
const LCD_RETURNHOME: u8 = 0x02;
const LCD_ENTRYMODESET: u8 = 0x04;
const LCD_DISPLAYCONTROL: u8 = 0x08;
const LCD_CURSORSHIFT: u8 = 0x10;
const LCD_FUNCTIONSET: u8 = 0x20;
const LCD_SETCGRAMADDR: u8 = 0x40;
const LCD_SETDDRAMADDR: u8 = 0x80;

// Flags for display entry mode
const LCD_ENTRYLEFT: u8 = 0x02;
const LCD_ENTRYSHIFTINCREMENT: u8 = 0x01;
const LCD_ENTRYSHIFTDECREMENT: u8 = 0x00;

// Flags for display on/off control
const LCD_DISPLAYON: u8 = 0x04;
const LCD_CURSORON: u8 = 0x02;
const LCD_CURSOROFF: u8 = 0x00;
const LCD_BLINKON: u8 = 0x01;
const LCD_BLINKOFF: u8 = 0x00;

// Flags for display/cursor shift
const LCD_DISPLAYMOVE: u8 = 0x08;
const LCD_MOVERIGHT: u8 = 0x04;
const LCD_MOVELEFT: u8 = 0x00;

// Flags for function set
const LCD_4BITMODE: u8 = 0x00;
const LCD_2LINE: u8 = 0x08;
const LCD_5X8_DOTS: u8 = 0x00;

// Pin definitions for LCD backpack
const RS_PIN: u8 = 0; // Register Select
const RW_PIN: u8 = 1; // Read/Write
const EN_PIN: u8 = 2; // Enable
const BL_PIN: u8 = 3; // Backlight
// Data pins are 4-7

pub struct OutputState {
    rs: bool,
    rw: bool,
    en: bool,
    backlight: bool,
    data: u8,
}

impl OutputState {
    fn new() -> Self {
        Self {
            rs: false,
            rw: false,
            en: false,
            backlight: true,
            data: 0,
        }
    }

    fn get_value(&self) -> u8 {
        let mut value = 0;

        if self.rs {
            value |= 1 << RS_PIN;
        }
        if self.rw {
            value |= 1 << RW_PIN;
        }
        if self.en {
            value |= 1 << EN_PIN;
        }
        if self.backlight {
            value |= 1 << BL_PIN;
        }

        // Data pins are the high 4 bits
        value |= self.data & 0xF0;

        value
    }
}

pub struct LcdI2c<I2C> {
    i2c: I2C,
    address: u8,
    output: OutputState,
    display_control: u8,
    display_function: u8,
    display_mode: u8,
    rows: u8,
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
            display_control: 0,
            display_function: 0,
            display_mode: 0,
            rows: 2,
        }
    }

    pub async fn begin(&mut self) -> Result<(), E> {
        // Wait for more than 15ms after VCC rises to 4.5V (datasheet)
        Timer::after_millis(50).await;

        self.output.backlight = true;
        self.write_pins()?;

        // Start in 8-bit mode, try to set 4-bit mode

        self.write4bits(0x03).await?;
        Timer::after_millis(5).await;

        self.write4bits(0x03).await?;
        Timer::after_millis(5).await;

        self.write4bits(0x03).await?;
        Timer::after_millis(1).await;

        // Finally, set to 4-bit interface
        self.write4bits(0x02).await?;
        Timer::after_millis(1).await;

        // Set # lines, font size, etc.
        self.display_function = LCD_4BITMODE | LCD_2LINE | LCD_5X8_DOTS;
        self.command(LCD_FUNCTIONSET | self.display_function)
            .await?;

        // Turn the display on with no cursor or blinking default
        self.display_control = LCD_DISPLAYON | LCD_CURSOROFF | LCD_BLINKOFF;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await?;

        // Clear display
        self.clear().await?;

        // Initialize to default text direction (for languages that read left to right)
        self.display_mode = LCD_ENTRYLEFT | LCD_ENTRYSHIFTDECREMENT;
        self.command(LCD_ENTRYMODESET | self.display_mode).await?;

        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), E> {
        self.command(LCD_CLEARDISPLAY).await?;
        Timer::after_millis(2).await;
        Ok(())
    }

    pub async fn home(&mut self) -> Result<(), E> {
        self.command(LCD_RETURNHOME).await?;
        Timer::after_millis(2).await;
        Ok(())
    }

    pub async fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), E> {
        let row_offsets: [u8; 4] = [0x00, 0x40, 0x14, 0x54];
        let row_idx = if row >= self.rows { self.rows - 1 } else { row };

        self.command(LCD_SETDDRAMADDR | (col + row_offsets[row_idx as usize]))
            .await
    }

    pub async fn no_display(&mut self) -> Result<(), E> {
        self.display_control &= !LCD_DISPLAYON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn display(&mut self) -> Result<(), E> {
        self.display_control |= LCD_DISPLAYON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn no_cursor(&mut self) -> Result<(), E> {
        self.display_control &= !LCD_CURSORON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn cursor(&mut self) -> Result<(), E> {
        self.display_control |= LCD_CURSORON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn no_blink(&mut self) -> Result<(), E> {
        self.display_control &= !LCD_BLINKON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn blink(&mut self) -> Result<(), E> {
        self.display_control |= LCD_BLINKON;
        self.command(LCD_DISPLAYCONTROL | self.display_control)
            .await
    }

    pub async fn scroll_display_left(&mut self) -> Result<(), E> {
        self.command(LCD_CURSORSHIFT | LCD_DISPLAYMOVE | LCD_MOVELEFT)
            .await
    }

    pub async fn scroll_display_right(&mut self) -> Result<(), E> {
        self.command(LCD_CURSORSHIFT | LCD_DISPLAYMOVE | LCD_MOVERIGHT)
            .await
    }

    pub async fn left_to_right(&mut self) -> Result<(), E> {
        self.display_mode |= LCD_ENTRYLEFT;
        self.command(LCD_ENTRYMODESET | self.display_mode).await
    }

    pub async fn right_to_left(&mut self) -> Result<(), E> {
        self.display_mode &= !LCD_ENTRYLEFT;
        self.command(LCD_ENTRYMODESET | self.display_mode).await
    }

    pub async fn autoscroll(&mut self) -> Result<(), E> {
        self.display_mode |= LCD_ENTRYSHIFTINCREMENT;
        self.command(LCD_ENTRYMODESET | self.display_mode).await
    }

    pub async fn no_autoscroll(&mut self) -> Result<(), E> {
        self.display_mode &= !LCD_ENTRYSHIFTINCREMENT;
        self.command(LCD_ENTRYMODESET | self.display_mode).await
    }

    pub fn backlight(&mut self) -> Result<(), E> {
        self.output.backlight = true;
        self.write_pins()
    }

    pub fn no_backlight(&mut self) -> Result<(), E> {
        self.output.backlight = false;
        self.write_pins()
    }

    pub async fn create_char(&mut self, location: u8, charmap: &[u8; 8]) -> Result<(), E> {
        let location = location & 0x7; // We only have 8 CGRAM locations (0-7)
        self.command(LCD_SETCGRAMADDR | (location << 3)).await?;

        for &b in charmap {
            self.write(b).await?;
        }

        Ok(())
    }

    pub async fn write_byte(&mut self, byte: u8) -> Result<(), E> {
        self.write(byte).await
    }

    pub async fn write_str(&mut self, s: &str) -> Result<(), E> {
        for b in s.bytes() {
            self.write(b).await?;
        }
        Ok(())
    }

    async fn command(&mut self, value: u8) -> Result<(), E> {
        self.output.rs = false;
        self.send(value).await
    }

    async fn write(&mut self, value: u8) -> Result<(), E> {
        self.output.rs = true;
        self.send(value).await
    }

    async fn send(&mut self, value: u8) -> Result<(), E> {
        // Send high nibble
        self.write4bits(value >> 4).await?;
        // Send low nibble
        self.write4bits(value & 0x0F).await?;

        Timer::after_micros(100).await;

        Ok(())
    }

    async fn write4bits(&mut self, value: u8) -> Result<(), E> {
        self.output.data = value << 4; // Move to high nibble position (bits 4-7)

        self.pulse_enable().await?;

        Ok(())
    }

    async fn pulse_enable(&mut self) -> Result<(), E> {
        self.output.en = false;
        self.write_pins()?;
        Timer::after_micros(1).await;

        self.output.en = true;
        self.write_pins()?;
        Timer::after_micros(1).await;

        self.output.en = false;
        self.write_pins()?;
        Timer::after_micros(100).await;

        Ok(())
    }

    fn write_pins(&mut self) -> Result<(), E> {
        let value = self.output.get_value();
        self.i2c.write(self.address, &[value])
    }
}

impl<I2C, E> fmt::Write for LcdI2c<I2C>
where
    I2C: I2c<Error = E>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.output.rs = true;

            // Send high nibble
            self.output.data = b & 0xF0;
            self.output.en = false;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }

            self.output.en = true;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }

            self.output.en = false;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }

            // Send low nibble
            self.output.data = (b & 0x0F) << 4;
            self.output.en = false;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }

            self.output.en = true;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }

            self.output.en = false;
            if self.write_pins().is_err() {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}
