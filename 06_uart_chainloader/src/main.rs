// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

// Rust embedded logo for `make doc`.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/rust-embedded/wg/master/assets/logo/ewg-logo-blue-white-on-transparent.png"
)]

//! The `kernel` binary.
//!
//! # Code organization and architecture
//!
//! The code is divided into different *modules*, each representing a typical **subsystem** of the
//! `kernel`. Top-level module files of subsystems reside directly in the `src` folder. For example,
//! `src/memory.rs` contains code that is concerned with all things memory management.
//!
//! ## Visibility of processor architecture code
//!
//! Some of the `kernel`'s subsystems depend on low-level code that is specific to the target
//! processor architecture. For each supported processor architecture, there exists a subfolder in
//! `src/_arch`, for example, `src/_arch/aarch64`.
//!
//! The architecture folders mirror the subsystem modules laid out in `src`. For example,
//! architectural code that belongs to the `kernel`'s MMU subsystem (`src/memory/mmu.rs`) would go
//! into `src/_arch/aarch64/memory/mmu.rs`. The latter file is loaded as a module in
//! `src/memory/mmu.rs` using the `path attribute`. Usually, the chosen module name is the generic
//! module's name prefixed with `arch_`.
//!
//! For example, this is the top of `src/memory/mmu.rs`:
//!
//! ```
//! #[cfg(target_arch = "aarch64")]
//! #[path = "../_arch/aarch64/memory/mmu.rs"]
//! mod arch_mmu;
//! ```
//!
//! Often times, items from the `arch_ module` will be publicly reexported by the parent module.
//! This way, each architecture specific module can provide its implementation of an item, while the
//! caller must not be concerned which architecture has been conditionally compiled.
//!
//! ## BSP code
//!
//! `BSP` stands for Board Support Package. `BSP` code is organized under `src/bsp.rs` and contains
//! target board specific definitions and functions. These are things such as the board's memory map
//! or instances of drivers for devices that are featured on the respective board.
//!
//! Just like processor architecture code, the `BSP` code's module structure tries to mirror the
//! `kernel`'s subsystem modules, but there is no reexporting this time. That means whatever is
//! provided must be called starting from the `bsp` namespace, e.g. `bsp::driver::driver_manager()`.
//!
//! ## Kernel interfaces
//!
//! Both `arch` and `bsp` contain code that is conditionally compiled depending on the actual target
//! and board for which the kernel is compiled. For example, the `interrupt controller` hardware of
//! the `Raspberry Pi 3` and the `Raspberry Pi 4` is different, but we want the rest of the `kernel`
//! code to play nicely with any of the two without much hassle.
//!
//! In order to provide a clean abstraction between `arch`, `bsp` and `generic kernel code`,
//! `interface` traits are provided *whenever possible* and *where it makes sense*. They are defined
//! in the respective subsystem module and help to enforce the idiom of *program to an interface,
//! not an implementation*. For example, there will be a common IRQ handling interface which the two
//! different interrupt controller `drivers` of both Raspberrys will implement, and only export the
//! interface to the rest of the `kernel`.
//!
//! ```
//!         +-------------------+
//!         | Interface (Trait) |
//!         |                   |
//!         +--+-------------+--+
//!            ^             ^
//!            |             |
//!            |             |
//! +----------+--+       +--+----------+
//! | kernel code |       |  bsp code   |
//! |             |       |  arch code  |
//! +-------------+       +-------------+
//! ```
//!
//! # Summary
//!
//! For a logical `kernel` subsystem, corresponding code can be distributed over several physical
//! locations. Here is an example for the **memory** subsystem:
//!
//! - `src/memory.rs` and `src/memory/**/*`
//!   - Common code that is agnostic of target processor architecture and `BSP` characteristics.
//!     - Example: A function to zero a chunk of memory.
//!   - Interfaces for the memory subsystem that are implemented by `arch` or `BSP` code.
//!     - Example: An `MMU` interface that defines `MMU` function prototypes.
//! - `src/bsp/__board_name__/memory.rs` and `src/bsp/__board_name__/memory/**/*`
//!   - `BSP` specific code.
//!   - Example: The board's memory map (physical addresses of DRAM and MMIO devices).
//! - `src/_arch/__arch_name__/memory.rs` and `src/_arch/__arch_name__/memory/**/*`
//!   - Processor architecture specific code.
//!   - Example: Implementation of the `MMU` interface for the `__arch_name__` processor
//!     architecture.
//!
//! From a namespace perspective, **memory** subsystem code lives in:
//!
//! - `crate::memory::*`
//! - `crate::bsp::memory::*`
//!
//! # Boot flow
//!
//! 1. The kernel's entry point is the function `cpu::boot::arch_boot::_start()`.
//!     - It is implemented in `src/_arch/__arch_name__/cpu/boot.s`.
//! 2. Once finished with architectural setup, the arch code calls `kernel_init()`.

#![allow(clippy::upper_case_acronyms)]
#![feature(asm_const)]
#![feature(format_args_nl)]
#![feature(panic_info_message)]
#![feature(trait_alias)]
#![no_main]
#![no_std]

mod bsp;
mod console;
mod cpu;
mod driver;
mod panic_wait;
mod print;
mod synchronization;

/// Early init code.
///
/// # Safety
///
/// - Only a single core must be active and running this function.
/// - The init calls in this function must appear in the correct order.
unsafe fn kernel_init() -> ! {
    // Initialize the BSP driver subsystem.
    if let Err(x) = bsp::driver::init() {
        panic!("Error initializing BSP driver subsystem: {}", x);
    }

    // Initialize all device drivers.
    driver::driver_manager().init_drivers();
    // println! is usable from here on.

    // Transition from unsafe to safe.
    kernel_main()
}

const MINILOAD_LOGO: &str = r#"
 __  __ _      _ _                 _
|  \/  (_)_ _ (_) |   ___  __ _ __| |
| |\/| | | ' \| | |__/ _ \/ _` / _` |
|_|  |_|_|_||_|_|____\___/\__,_\__,_|
"#;

/// The main function running after the early init.
fn kernel_main() -> ! {
    use console::console;

    // 相对上一节而言没有太多变化
    // 此处只是打印的log不同而已
    println!("{}", MINILOAD_LOGO);
    println!("{:^37}", bsp::board_name());
    println!();
    println!("[ML] Requesting binary");
    console().flush();

    // Discard any spurious received characters before starting with the loader protocol.
    console().clear_rx();

    // Notify `Minipush` to send the binary.
    for _ in 0..3 {
        console().write_char(3 as char);
    }

    // TIPS: 很明显, uart不仅仅是可以读取我们通过键盘输入的字符
    // 外部程序也通过uart端口发送数据
    // Read the binary's size.
    // 这里是阻塞读, 连续读4次, 可以拼接成 binary size
    let mut size: u32 = u32::from(console().read_char() as u8);
    size |= u32::from(console().read_char() as u8) << 8;
    size |= u32::from(console().read_char() as u8) << 16;
    size |= u32::from(console().read_char() as u8) << 24;

    // Trust it's not too big.
    console().write_char('O');
    console().write_char('K');

    // 获取board的加载地址, 对于rpi3, 此值 0x80000
    let kernel_addr: *mut u8 = bsp::memory::board_default_load_addr() as *mut u8;
    unsafe {
        // 通过串口逐字节地读取, 并写入到 kernel_addr
        // 注意是通过 volatile 指针来操作内存, 避免编译器优化
        // 同时, 读取size个字节, 不会多读
        for i in 0..size {
            core::ptr::write_volatile(kernel_addr.offset(i as isize), console().read_char() as u8)
        }
    }

    println!("[ML] Loaded! Executing the payload now\n");
    console().flush();

    // Use black magic to create a function pointer.
    // 这里是通过 transmute 将 kernel_addr 转换成一个函数指针, 然后调用它
    // 因为该地址存放的就是可执行的程序
    let kernel: fn() -> ! = unsafe { core::mem::transmute(kernel_addr) };

    // Jump to loaded kernel!
    kernel()
}
