// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! BSP driver support.

// 该mod提供了外设的内存映射
use super::memory::map::mmio;
use crate::{bsp::device_driver, console, driver as generic_driver};
use core::sync::atomic::{AtomicBool, Ordering};

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------
// UART 和 GPIO 的全局变量实例
// 在这里传入外设对应的正确的地址
static PL011_UART: device_driver::PL011Uart =
    unsafe { device_driver::PL011Uart::new(mmio::PL011_UART_START) };
static GPIO: device_driver::GPIO = unsafe { device_driver::GPIO::new(mmio::GPIO_START) };

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

/// 仅在成功初始化UART驱动后调用
fn post_init_uart() -> Result<(), &'static str> {
    // 将pl011 uart实例,注册到console
    console::register_console(&PL011_UART);

    Ok(())
}

/// 仅在成功初始化GPIO驱动后调用
fn post_init_gpio() -> Result<(), &'static str> {
    // 初始化pl011 uart要用到的GPIO
    GPIO.map_pl011_uart();
    Ok(())
}

/// 向device manager注册UART驱动
fn driver_uart() -> Result<(), &'static str> {
    let uart_descriptor =
        generic_driver::DeviceDriverDescriptor::new(&PL011_UART, Some(post_init_uart));
    generic_driver::driver_manager().register_driver(uart_descriptor);

    Ok(())
}

/// 向device manager注册GPIO驱动
fn driver_gpio() -> Result<(), &'static str> {
    let gpio_descriptor = generic_driver::DeviceDriverDescriptor::new(&GPIO, Some(post_init_gpio));
    generic_driver::driver_manager().register_driver(gpio_descriptor);

    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// 初始化驱动子系统
///
/// # Safety
///
/// See child function calls.
pub unsafe fn init() -> Result<(), &'static str> {
    // 确保初始化只执行一次
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err("Init already done");
    }

    // 注册uart和gpio驱动到device manager
    driver_uart()?;
    driver_gpio()?;

    // 初始化完成
    INIT_DONE.store(true, Ordering::Relaxed);
    Ok(())
}
