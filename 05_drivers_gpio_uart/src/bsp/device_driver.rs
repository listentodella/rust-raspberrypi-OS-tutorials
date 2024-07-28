// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! Device driver.

#[cfg(any(feature = "bsp_rpi3", feature = "bsp_rpi4"))]
// 该mod提供了bcm的外设驱动, 不过目前只定义了GPIO和UART驱动
mod bcm;
// 该mod则提供了用于访问外设的通用方法
// 实际上, 当前提供了安全访问MMIO的封装
mod common;

#[cfg(any(feature = "bsp_rpi3", feature = "bsp_rpi4"))]
pub use bcm::*;
