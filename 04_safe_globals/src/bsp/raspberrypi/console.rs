// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! BSP console facilities.

use crate::{console, synchronization, synchronization::NullLock};
// 注意这里的fmt是属于core库的
use core::fmt;

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

/// 用于记录通过console共写入了多少字节
/// 由于它是全局的, 访问它需要注意竞态问题
struct QEMUOutputInner {
    chars_written: usize,
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// 现在QEMUOutput 不再是一个空的结构体了
/// inner用一个NullLock封装QEMUOutputInner
/// 这样就可以保证在访问QEMUOutputInner时, 不会有竞态问题
pub struct QEMUOutput {
    inner: NullLock<QEMUOutputInner>,
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

/// 定义一个全局的、静态的QEMUOutput实例
static QEMU_OUTPUT: QEMUOutput = QEMUOutput::new();

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

impl QEMUOutputInner {
    /// 函数签名前如果有const关键字，意味着该函数是一个常量函数（const
    /// function）。常量函数可以在编译时被求值，这意味着它们的返回值可以在编译时确定
    /// 并且可以用于常量上下文中，例如在常量初始化中使用。
    const fn new() -> QEMUOutputInner {
        QEMUOutputInner { chars_written: 0 }
    }

    /// 发送一个字符
    fn write_char(&mut self, c: char) {
        unsafe {
            core::ptr::write_volatile(0x3F20_1000 as *mut u8, c as u8);
        }

        // 记录写入的字符数
        self.chars_written += 1;
    }
}

/// Implementing `core::fmt::Write` enables usage of the `format_args!` macros, which in turn are
/// used to implement the `kernel`'s `print!` and `println!` macros. By implementing `write_str()`,
/// we get `write_fmt()` automatically.
///
/// The function takes an `&mut self`, so it must be implemented for the inner struct.
///
/// See [`src/print.rs`].
///
/// [`src/print.rs`]: ../../print/index.html
// 注意这里的fmt是属于core库的
impl fmt::Write for QEMUOutputInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            // Convert newline to carrige return + newline.
            if c == '\n' {
                self.write_char('\r')
            }

            self.write_char(c);
        }

        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl QEMUOutput {
    /// 创建一个 `QEMUOutput` 实例.
    pub const fn new() -> QEMUOutput {
        QEMUOutput {
            /// 实质上要创建一个NullLock, 并将QEMUOutputInner的实例放入其中
            inner: NullLock::new(QEMUOutputInner::new()),
        }
    }
}

/// 获取console的一个静态引用, 注意这个All的trait是在另一处定义的
pub fn console() -> &'static dyn console::interface::All {
    &QEMU_OUTPUT
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------
/// 这是一个自己实现的用于同步的mod
use synchronization::interface::Mutex;

/// 为`QEMUOutput`实现`console::interface::Write` trait
impl console::interface::Write for QEMUOutput {
    /// 通过`Mutex`保护来序列化访问，将`args`传递给`core::fmt::Write`的实现。
    fn write_fmt(&self, args: core::fmt::Arguments) -> fmt::Result {
        // 通过`inner`的`lock()`方法"独占"访问`QEMUOutputInner`，
        // 并调用`core::fmt::Write::write_fmt()`方法来返回`fmt::Result`

        // 并提供闭包来访问`inner`
        self.inner.lock(|inner| fmt::Write::write_fmt(inner, args))
    }
}

/// 为`QEMUOutput`实现 `console::interface::Statistics` trait
impl console::interface::Statistics for QEMUOutput {
    // 通过`inner`的`lock()`方法"独占"访问`QEMUOutputInner`，
    // 并提供闭包来访问`inner`, 可以安全获得当前写入的字节数
    fn chars_written(&self) -> usize {
        self.inner.lock(|inner| inner.chars_written)
    }
}

// 为`QEMUOutput`实现`console::interface::All` trait
// `All` trait 其实就是上面两处trait实现的组合
impl console::interface::All for QEMUOutput {}
