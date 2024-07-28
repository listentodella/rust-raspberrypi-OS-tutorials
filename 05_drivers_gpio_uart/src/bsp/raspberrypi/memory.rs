// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! BSP Memory Management.

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// 根据板子,定义外设的地址映射
#[rustfmt::skip]
pub(super) mod map {

    // GPIO 的 offset
    pub const GPIO_OFFSET:         usize = 0x0020_0000;
    // UART 的 offset
    pub const UART_OFFSET:         usize = 0x0020_1000;

    /// Physical devices.
    #[cfg(feature = "bsp_rpi3")]
    pub mod mmio {
        use super::*;

        // 对于 rpi3, 起始地址是 0x3F00_0000
        pub const START:            usize =         0x3F00_0000;
        // rpi3 GPIO的地址
        pub const GPIO_START:       usize = START + GPIO_OFFSET;
        // rpi3 UART的地址
        pub const PL011_UART_START: usize = START + UART_OFFSET;
    }

    /// Physical devices.
    #[cfg(feature = "bsp_rpi4")]
    pub mod mmio {
        use super::*;

        pub const START:            usize =         0xFE00_0000;
        pub const GPIO_START:       usize = START + GPIO_OFFSET;
        pub const PL011_UART_START: usize = START + UART_OFFSET;
    }
}
