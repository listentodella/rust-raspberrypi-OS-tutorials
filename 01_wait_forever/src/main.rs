// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

// Rust embedded logo for `make doc`.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/rust-embedded/wg/master/assets/logo/ewg-logo-blue-white-on-transparent.png"
)]

//! The `kernel` binary.
//!
//! # 代码组织与架构
//!
//! 代码被分为多个 *modules*, 每一个都是`kernel`的一个典型的 **subsystem** .
//! 子系统的顶级模块文件直接位于src文件夹中。
//! 例如，src/memory.rs包含与内存管理相关的所有内容的代码。
//!
//! ## 处理器架构代码的可见性
//! 内核的某些子系统依赖于针对目标处理器架构的特定底层代码。
//! 对于每个支持的处理器架构，在`src/_arch`中都存在一个子文件夹，例如`src/_arch/aarch64`。
//! 架构文件夹反映了在`src`中布局的子系统模块。例如，属于内核MMU子系统（`src/memory/mmu.
//! rs`）的架构代码将位于`src/_arch/aarch64/memory/mmu.rs`。 后者文件作为模块在`src/memory/mmu.
//! rs`中使用`path`属性加载。通常，选定的模块名称是以`arch_`为前缀的通用模块名称。
//! 例如，这是`src/memory/mmu.rs`的顶部内容：
//!
//! ```
//! #[cfg(target_arch = "aarch64")]
//! #[path = "../_arch/aarch64/memory/mmu.rs"]
//! mod arch_mmu;
//! ```
//!
//! 通常情况下，来自`arch_`模块的元素会被其父模块公开地重新导出。这样，
//! 每个特定于架构的模块都可以提供某个元素的实现，而调用者则无需关心哪个架构已经被条件编译。
//!
//! ## BSP code
//!
//! BSP代码组织在`src/bsp.rs`下，并包含针对目标板卡的特定定义和函数。
//! 这些内容包括板卡的内存映射或特定于板卡上设备的驱动程序实例等。 与处理器架构代码类似，
//! BSP代码的模块结构试图与内核的子系统模块相对应，但这次并没有重新导出。
//! 这意味着所提供的任何内容都必须从bsp命名空间开始调用，例如`bsp::driver::driver_manager()`。
//!
//! ## Kernel interfaces
//!
//! `arch`和`bsp`都包含根据实际目标和板卡条件编译的代码。内核为这些目标和板卡编译时，
//! 会选择性地编译这些代码。例如，RPi3 和 RPi4
//! 中断控制器硬件是不同的，但我们希望内核的其余部分能够轻松地与两者中的任何一个协同工作，
//! 而无需太多麻烦。

//! 为了在`arch`、`bsp`和通用内核代码之间提供一个清晰的抽象，*当可能且合理时*，
//! 会提供`interface` trait。 这些接口特性定义在各自的子系统模块中，有助于强制实现“面向接口编程，
//! 而不是面向实现”的编程范式。 例如，将有一个通用的IRQ处理接口，Raspberry Pi 3和Raspberry Pi
//! 4的两个不同的中断控制器驱动程序将实现这个接口，并仅将接口导出给内核的其余部分。
//
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
//! # 总结
//! 对于逻辑内核子系统，相应的代码可以分布在多个物理位置。以下是内存子系统的一个例子：
//! - `src/memory.rs`` 和 `src/memory/**/*`
//!   - 与目标处理器架构和BSP特性无关的通用代码。
//!     - 示例：一个用于将内存块清零的函数。
//!
//! - 由`arch`或`BSP`代码实现的内存子系统的接口。
//!   - 示例：一个定义了`MMU`函数原型的`MMU`接口。
//! - `src/bsp/__board_name__/memory.rs` 和 `src/bsp/__board_name__/memory/**/*`
//!   - 针对特定BSP的代码。
//!   - 示例：板的内存映射（DRAM和MMIO设备的物理地址）。
//!
//! - `src/_arch/__arch_name__/memory.rs` 和 `src/_arch/__arch_name__/memory/**/*`
//!   - 针对特定处理器架构的代码。
//!   - 示例：针对__arch_name__处理器架构的MMU接口实现。
//!
//! 从命名空间的角度来看，内存子系统代码位于：
//! - `crate::memory::*`
//! - `crate::bsp::memory::*`
//!
//! # Boot flow
//!
//! 1. The kernel's entry point is the function `cpu::boot::arch_boot::_start()`.
//!     - It is implemented in `src/_arch/__arch_name__/cpu/boot.s`.

#![no_main]
#![no_std]

mod bsp;
mod cpu;
mod panic_wait;

// Kernel code coming next tutorial.
