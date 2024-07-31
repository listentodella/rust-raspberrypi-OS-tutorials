// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2021-2023 Andre Richter <andre.o.richter@gmail.com>

//! Architectural boot code.
//!
//! # Orientation
//!
//! Since arch modules are imported into generic modules using the path attribute, the path of this
//! file is:
//!
//! crate::cpu::boot::arch_boot

use aarch64_cpu::{asm, registers::*};
use core::arch::global_asm;
use tock_registers::interfaces::Writeable;

// Assembly counterpart to this file.
global_asm!(
    include_str!("boot.s"),
    CONST_CURRENTEL_EL2 = const 0x8,
    CONST_CORE_ID_MASK = const 0b11
);

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

/// 准备从el2切换到el1
///
/// # Safety
///
/// - `bss`段还未初始化. 代码不能使用或引用它.
/// - The HW state of EL1 must be prepared in a sound way.
#[inline(always)]
unsafe fn prepare_el2_to_el1_transition(phys_boot_core_stack_end_exclusive_addr: u64) {
    // Enable timer counter registers for EL1.
    // 允许EL1访问timer
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);

    // No offset for reading the counters.
    CNTVOFF_EL2.set(0);

    // 确保EL1的执行状态为AArch64
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // 建立一个模拟的异常返回
    //
    // 首先, 创建一个假的 saved pragram status
    // - 所有的interrupts处于屏蔽状态, 并且选用SP_EL1作为sp
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    // 接着, 设置 ELR 为 kernel_init(), 这样异常返回后, 会紧接着该函数执行
    ELR_EL2.set(crate::kernel_init as *const () as u64);

    // 设置SP_EL1, 它会在我们返回到EL1时, 作为我们的sp
    // 由于此前到达EL2时也没做什么特别的事情, 因此我们在这里就直接复用相同的栈吧
    SP_EL1.set(phys_boot_core_stack_end_exclusive_addr);
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// The Rust entry of the `kernel` binary.
///
/// The function is called from the assembly `_start` function.
///
/// # Safety
///
/// - Exception return from EL2 must must continue execution in EL1 with `kernel_init()`.
#[no_mangle]
pub unsafe extern "C" fn _start_rust(phys_boot_core_stack_end_exclusive_addr: u64) -> ! {
    // 这里的入参并不是平时通过函数调用直接传入的
    // aarch64 的准则是使用x0...开始的若干寄存器,依次作为参数
    // 因此这里的入参来自于 x0 寄存器, 而在此前, x0已备份到sp中
    prepare_el2_to_el1_transition(phys_boot_core_stack_end_exclusive_addr);

    // 上面函数的返回并不会发生什么特别的事情
    // 只有使用 `eret` 指令, 才能从异常返回
    // 这里会返回到EL1, 并且由于上面的ELR设置为了 `kernel_init()`, 因此会执行kernel_init
    // 并且由于SP_EL1设置了栈指针, 因此kernel_init会使用该sp去执行
    asm::eret()
}
