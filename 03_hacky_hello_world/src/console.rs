// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! System console.

use crate::bsp;

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Console interfaces.
pub mod interface {
    /// Console write functions.
    ///
    /// `core::fmt::Write` 确实是我们当前所需的.
    /// 在这里重导出它, 因为实现 `console::Write` 给读者一个更明确的提示.
    pub use core::fmt::Write;
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// 返回一个对console的引用.
///
/// 这是所有打印宏所使用的全局console.
pub fn console() -> impl interface::Write {
    bsp::console::console()
}
