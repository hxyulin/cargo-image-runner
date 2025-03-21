//! A basic serial port implementation.
//! From https://gist.github.com/hxyulin/d9dc1b9eabc3d1e762a0a71e19984803

use core::arch::asm;

pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const fn new_uninitialized(port: u16) -> Self {
        Self { port }
    }

    /// # Safety
    ///
    /// This function is unsafe because it does not check if the serial port is initialized.
    pub unsafe fn new(port: u16) -> Self {
        let mut serial_port = Self::new_uninitialized(port);
        unsafe { serial_port.init() };
        serial_port
    }

    #[inline]
    fn int_en(&self) -> u16 {
        self.port + 1
    }

    #[inline]
    fn fifo(&self) -> u16 {
        self.port + 2
    }

    #[inline]
    fn line_ctrl(&self) -> u16 {
        self.port + 3
    }

    #[inline]
    fn modem_ctrl(&self) -> u16 {
        self.port + 4
    }

    #[inline]
    fn line_sts(&self) -> u16 {
        self.port + 5
    }

    /// # Safety
    ///
    /// This function is unsafe because it does not check if the serial port is initialized.
    pub unsafe fn init(&mut self) {
        unsafe {
            // Disable interrupts
            outb(self.int_en(), 0);

            // Enable DLAB (set baud rate divisor)
            outb(self.line_ctrl(), 0x80);

            // Set divisor to 3 (lo byte) 38400 baud
            outb(self.port + 0, 0x03);
            outb(self.int_en(), 0x00); // Disable all interrupts

            // 8 bits, no parity, one stop bit
            outb(self.line_ctrl(), 0x03);

            // Enable FIFO, clear them, with 14-byte threshold
            outb(self.fifo(), 0xC7);

            // Ready to transmit
            outb(self.line_sts(), 0x0B);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        unsafe {
            while (inb(self.line_sts()) & 0x20) == 0 {}
            outb(self.port, byte);
        }
    }
}

impl Drop for SerialPort {
    fn drop(&mut self) {
        unsafe {
            outb(self.int_en(), 0);
            outb(self.fifo(), 0x00);
            outb(self.modem_ctrl(), 0x00);
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// # Safety
///
/// This function is unsafe because it is not checked if the port is valid.
#[inline]
unsafe fn outb(port: u16, data: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") data);
    }
}

/// # Safety
///
/// This function is unsafe because it is not checked if the port is valid.
#[inline]
unsafe fn inb(port: u16) -> u8 {
    unsafe {
        let mut data: u8;
        asm!("in al, dx", out("al") data, in("dx") port);
        data
    }
}
