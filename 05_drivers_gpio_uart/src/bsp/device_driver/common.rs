// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2020-2023 Andre Richter <andre.o.richter@gmail.com>

//! Common device driver code.
// PhantomData 只是个mark trait
// 由于rust不支持定义泛型却暂时不使用
// 所以通过PhantomData来告知编译器, 该泛型将来会用得上, 不要编译警告
// PhantomData奇妙的是, 它是一个zero-size的结构体, 即便它封装了什么, 也不占用任何空间
use core::{marker::PhantomData, ops};

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

pub struct MMIODerefWrapper<T> {
    // 被MMIO的设备的起始地址
    start_addr: usize,
    // 告诉编译器, 该泛型是一个函数指针, 其返回值类型为T
    phantom: PhantomData<fn() -> T>,
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl<T> MMIODerefWrapper<T> {
    /// 创建一个实例
    pub const unsafe fn new(start_addr: usize) -> Self {
        Self {
            start_addr,
            phantom: PhantomData,
        }
    }
}

// 实现 Deref trait, 使得 MMIODerefWrapper 实例可以像引用一样使用
impl<T> ops::Deref for MMIODerefWrapper<T> {
    type Target = T;

    // 通过该方法, 即可获取 MMIODerefWrapper 实例里的 T 值
    fn deref(&self) -> &Self::Target {
        // 将 start_addr 转换为 *const T 指针
        // 然后通过*获取该地址里的值
        // 最后通过&将其转换为&T
        unsafe { &*(self.start_addr as *const _) }
    }
}
