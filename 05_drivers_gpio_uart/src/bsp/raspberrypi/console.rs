// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! BSP console facilities.

use crate::console;

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// Return a reference to the console.
/// 获取一个console的引用
/// 注意从现在开始可以是真实硬件的console了
pub fn console() -> &'static dyn console::interface::All {
    &super::driver::PL011_UART
}
