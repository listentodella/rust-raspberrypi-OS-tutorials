// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! BSP console facilities.

use crate::console;
use core::fmt;

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

/// 这里是一个空的结构体, 实际上只是用于实现 `core::fmt::Write` trait
/// 之后就可以通过它调用该trait, 达到输出的目的
struct QEMUOutput;

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

/// Implementing `core::fmt::Write` enables usage of the `format_args!` macros, which in turn are
/// used to implement the `kernel`'s `print!` and `println!` macros.
/// 通过实现 `write_str()`, 可以自动实现 `write_fmt()`
///
/// See [`src/print.rs`].
///
/// [`src/print.rs`]: ../../print/index.html
impl fmt::Write for QEMUOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            unsafe {
                core::ptr::write_volatile(0x3F20_1000 as *mut u8, c as u8);
            }
        }

        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// 通过该函数可以得到 `console::interface::Write` trait 的实现, 即 `QEMUOutput`
/// 借助`QEMUOutput`就可以向console打印了
pub fn console() -> impl console::interface::Write {
    QEMUOutput {}
}
