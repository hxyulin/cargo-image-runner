#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(crate::tests::runner))]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

mod serial;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut serial_port = unsafe { serial::SerialPort::new(0x3F8) };
    #[cfg(test)]
    {
        serial_port.write_str("[fail]\n").unwrap();
        tests::exit_qemu(tests::ExitCode::Failed);
    }
    serial_port
        .write_fmt(format_args!("Kernel Panic: {}", info))
        .unwrap();
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[no_mangle]
extern "C" fn _start() -> ! {
    let mut serial_port = unsafe { serial::SerialPort::new(0x3F8) };
    serial_port.write_str("Hello, world!\n").unwrap();
    #[cfg(test)]
    {
        test_main();
    }
    panic!("End of program");
}

#[cfg(test)]
mod tests {
    use core::hint::unreachable_unchecked;

    use super::*;

    pub enum ExitCode {
        Success = 0x10,
        Failed = 0x11,
    }

    pub fn exit_qemu(exit_code: ExitCode) -> ! {
        unsafe { asm!("mov dx, 0xf4; mov eax, {:e}; out dx, eax", in(reg) exit_code as u32) };
        unsafe { unreachable_unchecked() }
    }

    pub trait Testable {
        fn name(&self) -> &str;
        fn run(&self);
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn name(&self) -> &str {
            core::any::type_name::<T>()
        }

        fn run(&self) {
            (self)()
        }
    }

    pub fn runner(tests: &[&dyn Testable]) {
        let mut serial_port = unsafe { serial::SerialPort::new(0x3F8) };
        for test in tests {
            serial_port
                .write_fmt(format_args!("Running test {}...  ", test.name()))
                .unwrap();
            test.run();
            serial_port.write_str("[ok]\n").unwrap();
        }
        exit_qemu(ExitCode::Success);
    }

    #[test_case]
    fn basic_test() {
        assert!(true);
    }
}
