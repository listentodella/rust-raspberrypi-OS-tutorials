// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! PL011 UART driver.
//!
//! # Resources
//!
//! - <https://github.com/raspberrypi/documentation/files/1888662/BCM2837-ARM-Peripherals.-.Revised.-.V2-1.pdf>
//! - <https://developer.arm.com/documentation/ddi0183/latest>

use crate::{
    bsp::device_driver::common::MMIODerefWrapper, console, cpu, driver, synchronization,
    synchronization::NullLock,
};
use core::fmt;
// 用于生成结构体RegisterBlock
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

// PL011 UART registers.
//
// Descriptions taken from "PrimeCell UART (PL011) Technical Reference Manual" r1p5.
register_bitfields! {
    u32,

    /// Flag Register.
    FR [
        /// Transmit FIFO empty. The meaning of this bit depends on the state of the FEN bit in the
        /// Line Control Register, LCR_H.
        ///
        /// - If the FIFO is disabled, this bit is set when the transmit holding register is empty.
        /// - If the FIFO is enabled, the TXFE bit is set when the transmit FIFO is empty.
        /// - This bit does not indicate if there is data in the transmit shift register.
        TXFE OFFSET(7) NUMBITS(1) [],

        /// Transmit FIFO full. The meaning of this bit depends on the state of the FEN bit in the
        /// LCR_H Register.
        ///
        /// - If the FIFO is disabled, this bit is set when the transmit holding register is full.
        /// - If the FIFO is enabled, the TXFF bit is set when the transmit FIFO is full.
        TXFF OFFSET(5) NUMBITS(1) [],

        /// Receive FIFO empty. The meaning of this bit depends on the state of the FEN bit in the
        /// LCR_H Register.
        ///
        /// - If the FIFO is disabled, this bit is set when the receive holding register is empty.
        /// - If the FIFO is enabled, the RXFE bit is set when the receive FIFO is empty.
        RXFE OFFSET(4) NUMBITS(1) [],

        /// UART busy. If this bit is set to 1, the UART is busy transmitting data. This bit remains
        /// set until the complete byte, including all the stop bits, has been sent from the shift
        /// register.
        ///
        /// This bit is set as soon as the transmit FIFO becomes non-empty, regardless of whether
        /// the UART is enabled or not.
        BUSY OFFSET(3) NUMBITS(1) []
    ],

    /// Integer Baud Rate Divisor.
    IBRD [
        /// The integer baud rate divisor.
        BAUD_DIVINT OFFSET(0) NUMBITS(16) []
    ],

    /// Fractional Baud Rate Divisor.
    FBRD [
        ///  The fractional baud rate divisor.
        BAUD_DIVFRAC OFFSET(0) NUMBITS(6) []
    ],

    /// Line Control Register.
    LCR_H [
        /// Word length. These bits indicate the number of data bits transmitted or received in a
        /// frame.
        #[allow(clippy::enum_variant_names)]
        WLEN OFFSET(5) NUMBITS(2) [
            FiveBit = 0b00,
            SixBit = 0b01,
            SevenBit = 0b10,
            EightBit = 0b11
        ],

        /// Enable FIFOs:
        ///
        /// 0 = FIFOs are disabled (character mode) that is, the FIFOs become 1-byte-deep holding
        /// registers.
        ///
        /// 1 = Transmit and receive FIFO buffers are enabled (FIFO mode).
        FEN  OFFSET(4) NUMBITS(1) [
            FifosDisabled = 0,
            FifosEnabled = 1
        ]
    ],

    /// Control Register.
    CR [
        /// Receive enable. If this bit is set to 1, the receive section of the UART is enabled.
        /// Data reception occurs for either UART signals or SIR signals depending on the setting of
        /// the SIREN bit. When the UART is disabled in the middle of reception, it completes the
        /// current character before stopping.
        RXE OFFSET(9) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],

        /// Transmit enable. If this bit is set to 1, the transmit section of the UART is enabled.
        /// Data transmission occurs for either UART signals, or SIR signals depending on the
        /// setting of the SIREN bit. When the UART is disabled in the middle of transmission, it
        /// completes the current character before stopping.
        TXE OFFSET(8) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],

        /// UART enable:
        ///
        /// 0 = UART is disabled. If the UART is disabled in the middle of transmission or
        /// reception, it completes the current character before stopping.
        ///
        /// 1 = The UART is enabled. Data transmission and reception occurs for either UART signals
        /// or SIR signals depending on the setting of the SIREN bit
        UARTEN OFFSET(0) NUMBITS(1) [
            /// If the UART is disabled in the middle of transmission or reception, it completes the
            /// current character before stopping.
            Disabled = 0,
            Enabled = 1
        ]
    ],

    /// Interrupt Clear Register.
    ICR [
        /// Meta field for all pending interrupts.
        ALL OFFSET(0) NUMBITS(11) []
    ]
}

// 该宏由第三方crate tock_registers提供，用于生成结构体RegisterBlock
register_structs! {
    #[allow(non_snake_case)]
    pub RegisterBlock {
        (0x00 => DR: ReadWrite<u32>),
        (0x04 => _reserved1),
        (0x18 => FR: ReadOnly<u32, FR::Register>),
        (0x1c => _reserved2),
        (0x24 => IBRD: WriteOnly<u32, IBRD::Register>),
        (0x28 => FBRD: WriteOnly<u32, FBRD::Register>),
        (0x2c => LCR_H: WriteOnly<u32, LCR_H::Register>),
        (0x30 => CR: WriteOnly<u32, CR::Register>),
        (0x34 => _reserved3),
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x48 => @END),
    }
}

/// 对与PL011 UART相关的MMIO寄存器的抽象
type Registers = MMIODerefWrapper<RegisterBlock>;

#[derive(PartialEq)]
enum BlockingMode {
    Blocking,
    NonBlocking,
}

// PL011 UART 驱动的内部描述, 重点在于Registers
struct PL011UartInner {
    registers: Registers,
    chars_written: usize,
    chars_read: usize,
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// NUllLock 的封装, 以使外部可以安全访问UART
pub struct PL011Uart {
    inner: NullLock<PL011UartInner>,
}

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

impl PL011UartInner {
    /// 创建UART实例
    ///
    /// # Safety
    ///
    /// - 用户必须确保提供正确的MMIO起始地址.
    pub const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            registers: Registers::new(mmio_start_addr),
            chars_written: 0,
            chars_read: 0,
        }
    }

    /// 设置波特率和其他特性
    ///
    /// 8N1 模式, 8位数据位, 无校验位, 1停止位.
    /// 波特率设置为  921600.
    ///
    /// BRD的计算公式如下, 其中CLK为48MHz(from config.txt):
    /// `(48_000_000 / 16) / 921_600 = 3.2552083`.
    ///
    /// 取其整数部分, 即`3`, 写入 `IBRD`寄存器(Integer Baud Rate Divisor).
    /// 取其小数部分, 即`0.2552083`, 写入 `FBRD`寄存器(Fractional Baud Rate)
    /// 不过, 根据PL011手册, 小数部分要乘以64, 再取其整数部分, 即`16`,
    /// `INTEGER((0.2552083 * 64) + 0.5) = 16`.
    ///
    /// 因此, 得到波特率分频: `3 + 16/64 = 3.25`
    /// 实际波特率会是 `48_000_000 / (16 * 3.25) = 923_077`.
    /// 波特率实际误差为 `((923_077 - 921_600) / 921_600) * 100 = 0.16%`.
    pub fn init(&mut self) {
        // 执行到这里时, 可能还有字符在TX FIFO中排队等待发送, 但UART硬件正处于发送状态.
        // 如果在这种情况下, UART被关闭, 那些排队等待发送的字符将会丢失.
        //
        // 比如, 在调用panic!()时, 可能发生这种情况. 因为 panic!()会初始化自己的UART实例,
        // 并调用init().
        // 因此, 先调用 flush() 确保所有待发送的字符都被发送出去.
        //
        // Hence, flush first to ensure all pending characters are transmitted.
        self.flush();

        // 临时关闭UART
        self.registers.CR.set(0);

        // 清除UART所有pending的中断
        self.registers.ICR.write(ICR::ALL::CLEAR);

        // 根据PL011手册:
        // LCR_H, IBRD, and FBRD registers 形成了一个30位宽的LCR寄存器,
        // 该寄存器在LCR_H寄存器被写时更新.
        // 因此, 为了更新 IBRD 或 FBRD 寄存器, 必须在最后进行一次LCR_H寄存器的写操作.
        //
        // Set the baud rate, 8N1 and FIFO enabled.
        self.registers.IBRD.write(IBRD::BAUD_DIVINT.val(3));
        self.registers.FBRD.write(FBRD::BAUD_DIVFRAC.val(16));
        self.registers
            .LCR_H
            .write(LCR_H::WLEN::EightBit + LCR_H::FEN::FifosEnabled);

        // 打开UART
        self.registers
            .CR
            .write(CR::UARTEN::Enabled + CR::TXE::Enabled + CR::RXE::Enabled);
    }

    /// 发送一个字符
    fn write_char(&mut self, c: char) {
        // 如果TX FIFO满了, 则一直循环等待空闲的位置.
        while self.registers.FR.matches_all(FR::TXFF::SET) {
            cpu::nop();
        }

        // 此时, TX FIFO肯定已经有空闲了, 因此可以直接写入
        self.registers.DR.set(c as u32);

        // 并更新统计信息
        self.chars_written += 1;
    }

    /// 阻塞执行, 直到最后一个待发送的字符被发送出去.(也许并不是发送完成?至少PL011安全空闲下来了)
    fn flush(&self) {
        // 循环让cpu执行空指令, 直到PL011的TX FIFO空标志被清除.
        while self.registers.FR.matches_all(FR::BUSY::SET) {
            cpu::nop();
        }
    }

    /// 获取一个字符.
    fn read_char_converting(&mut self, blocking_mode: BlockingMode) -> Option<char> {
        // 如果 RX FIFO 为空
        if self.registers.FR.matches_all(FR::RXFE::SET) {
            // 在非阻塞模式下, 立即返回None
            if blocking_mode == BlockingMode::NonBlocking {
                return None;
            }

            // 阻塞模式下, 则循环检查RX FIFO,直至不为空
            // 不过这种情况下, CPU只能被阻塞在这个函数上, 不能执行其他任务.
            while self.registers.FR.matches_all(FR::RXFE::SET) {
                cpu::nop();
            }
        }

        // 总之不管是否为阻塞模式,运行到这里, 都应该有字符在RX FIFO中了.
        // 从DR寄存器里读取一个字符
        let mut ret = self.registers.DR.get() as u8 as char;

        // Convert carrige return to newline.
        if ret == '\r' {
            ret = '\n'
        }

        // 更新统计信息
        self.chars_read += 1;

        // 用Some封装读取到的字符, 并返回.
        Some(ret)
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
impl fmt::Write for PL011UartInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl PL011Uart {
    // 定义UART驱动的兼容性字符串
    pub const COMPATIBLE: &'static str = "BCM PL011 UART";

    /// Create an instance.
    ///
    /// # Safety
    ///
    /// - 用户必须确保提供正确的MMIO起始地址.
    pub const unsafe fn new(mmio_start_addr: usize) -> Self {
        Self {
            inner: NullLock::new(PL011UartInner::new(mmio_start_addr)),
        }
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------
use synchronization::interface::Mutex;

impl driver::interface::DeviceDriver for PL011Uart {
    // 返回驱动的兼容性字符串
    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    // override init()方法, 用于初始化PL011 UART
    unsafe fn init(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.init());

        Ok(())
    }
}

/// 为PL011 UART实现console::interface::Write
/// 由于通过Mutex保护了PL011UartInner, 因此可认为是安全的
impl console::interface::Write for PL011Uart {
    /// Passthrough of `args` to the `core::fmt::Write` implementation, but guarded by a Mutex to
    /// serialize access.
    fn write_char(&self, c: char) {
        self.inner.lock(|inner| inner.write_char(c));
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> fmt::Result {
        // Fully qualified syntax for the call to `core::fmt::Write::write_fmt()` to increase
        // readability.
        self.inner.lock(|inner| fmt::Write::write_fmt(inner, args))
    }

    fn flush(&self) {
        // Spin until TX FIFO empty is set.
        self.inner.lock(|inner| inner.flush());
    }
}

/// 为PL011 UART实现console::interface::Read
/// 由于通过Mutex保护了PL011UartInner, 因此可认为是安全的
impl console::interface::Read for PL011Uart {
    // 以阻塞模式读取一个字符
    fn read_char(&self) -> char {
        self.inner
            .lock(|inner| inner.read_char_converting(BlockingMode::Blocking).unwrap())
    }

    fn clear_rx(&self) {
        // 实际上是以非阻塞模式读取的形式清空
        // 读到的数据并没有返回出去, 相当于读到后丢弃了
        while self
            .inner
            .lock(|inner| inner.read_char_converting(BlockingMode::NonBlocking))
            .is_some()
        {}
    }
}

/// 为PL011 UART实现console::interface::Statistics
/// 由于通过Mutex保护了PL011UartInner, 因此可认为是安全的
impl console::interface::Statistics for PL011Uart {
    fn chars_written(&self) -> usize {
        self.inner.lock(|inner| inner.chars_written)
    }

    fn chars_read(&self) -> usize {
        self.inner.lock(|inner| inner.chars_read)
    }
}

impl console::interface::All for PL011Uart {}
