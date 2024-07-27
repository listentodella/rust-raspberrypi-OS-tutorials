// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! System console.

use crate::bsp;

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Console 的接口
pub mod interface {
    use core::fmt;

    /// Console write functions.
    pub trait Write {
        /// Write a Rust format string.
        fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result;
    }

    /// Console的统计信息
    pub trait Statistics {
        /// 先提供了一个默认实现, 后续再根据需要override
        fn chars_written(&self) -> usize {
            0
        }
    }

    /// Trait alias for a full-fledged console.
    /// 相当于给Console定义了一个trait别名
    /// 很巧妙的写法, 明明一开始只是零散地定义了几个trait
    /// 后来将它们合并成了一个trait, 简洁明了, 又不需要重构代码
    pub trait All: Write + Statistics {}
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// Return a reference to the console.
///
/// This is the global console used by all printing macros.
pub fn console() -> &'static dyn interface::All {
    bsp::console::console()
}
