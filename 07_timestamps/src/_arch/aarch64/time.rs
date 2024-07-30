// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! 架构提供的定时器
//!
//! # Orientation
//!
//! Since arch modules are imported into generic modules using the path attribute, the path of this
//! file is:
//!
//! crate::time::arch_time

use crate::warn;
// 提供的aarch64架构下的一些指令,以及访问其寄存器的方法
use aarch64_cpu::{asm::barrier, registers::*};
use core::{
    num::{NonZeroU128, NonZeroU32, NonZeroU64},
    ops::{Add, Div},
    time::Duration,
};
use tock_registers::interfaces::Readable;

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

// 定义一个常量, ns与s之间的转换常数
const NANOSEC_PER_SEC: NonZeroU64 = NonZeroU64::new(1_000_000_000).unwrap();

#[derive(Copy, Clone, PartialOrd, PartialEq)]
struct GenericTimerCounterValue(u64);

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

/// boot.s 里 在rust代码得以执行前,会用 `CNTFRQ_EL0` 寄存器里的值覆盖该变量
/// 这里给出的这个值只是个(安全)的假值
#[no_mangle]
static ARCH_TIMER_COUNTER_FREQUENCY: NonZeroU32 = NonZeroU32::MIN;

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

// 获取当前cpu/架构下的定时器计数器的频率
fn arch_timer_counter_frequency() -> NonZeroU32 {
    // 使用 volatile 读取内存, 防止编译器优化 ARCH_TIMER_COUNTER_FREQUENCY
    // 因为仅从编译阶段看, 该变量就是上面的定义的值, 没有volatile的话, 很可能这句会直接被优化为数值
    // 实际上, 该值会在汇编里被更新
    unsafe { core::ptr::read_volatile(&ARCH_TIMER_COUNTER_FREQUENCY) }
}

impl GenericTimerCounterValue {
    pub const MAX: Self = GenericTimerCounterValue(u64::MAX);
}

impl Add for GenericTimerCounterValue {
    type Output = Self;

    // 为 GenericTimerCounterValue 实现 Add trait, 用于实现 + 操作
    // 这里可以得知, 使用 wrapping_add 避免溢出
    fn add(self, other: Self) -> Self {
        GenericTimerCounterValue(self.0.wrapping_add(other.0))
    }
}

// 为 Duration 实现 From<GenericTimerCounterValue> trait
// 用于将 GenericTimerCounterValue 转换为 Duration
impl From<GenericTimerCounterValue> for Duration {
    fn from(counter_value: GenericTimerCounterValue) -> Self {
        // 如果该数为0, 则返回 Duration::ZERO
        if counter_value.0 == 0 {
            return Duration::ZERO;
        }

        // 获取当前cpu/架构下的定时器计数器的频率
        // 需要注意的是, arch_timer_counter_frequency() 是一个 NonZeroU32
        // 在这里被转换为 NonZeroU64
        let frequency: NonZeroU64 = arch_timer_counter_frequency().into();

        // 由于 frequency 是一个 NonZeroU64, 所以这里使用除法是不用担心出错的
        let secs = counter_value.0.div(frequency);

        // 这里同样是安全的, 因为 frequency 实际永远不会大于 u32::MAX, 因为它原本是u32转换而来
        // 这意味着, sub_second_counter_value 最大理论值是 (u32::MAX - 1)
        // 因此, (sub_second_counter_value * NANOSEC_PER_SEC) 也不会溢出 u64
        //
        // The subsequent division ensures the result fits into u32, since the max result is smaller
        // than NANOSEC_PER_SEC. Therefore, just cast it to u32 using `as`.
        let sub_second_counter_value = counter_value.0 % frequency;
        let nanos = unsafe { sub_second_counter_value.unchecked_mul(u64::from(NANOSEC_PER_SEC)) }
            .div(frequency) as u32;

        // 将该数转换为 Duration
        Duration::new(secs, nanos)
    }
}

fn max_duration() -> Duration {
    Duration::from(GenericTimerCounterValue::MAX)
}

// 为 GenericTimerCounterValue 实现 TryFrom<Duration> trait
// 有可能失败
impl TryFrom<Duration> for GenericTimerCounterValue {
    type Error = &'static str;

    fn try_from(duration: Duration) -> Result<Self, Self::Error> {
        // 如果传入的duration小于timer的分辨率, 则返回0
        if duration < resolution() {
            return Ok(GenericTimerCounterValue(0));
        }

        // 如果传入的duration大于timer的最大值, 则返回错误
        if duration > max_duration() {
            return Err("Conversion error. Duration too big");
        }

        let frequency: u128 = u32::from(arch_timer_counter_frequency()) as u128;
        let duration: u128 = duration.as_nanos();

        // This is safe, because frequency can never be greater than u32::MAX, and
        // (Duration::MAX.as_nanos() * u32::MAX) < u128::MAX.
        let counter_value =
            unsafe { duration.unchecked_mul(frequency) }.div(NonZeroU128::from(NANOSEC_PER_SEC));

        // Since we checked above that we are <= max_duration(), just cast to u64.
        Ok(GenericTimerCounterValue(counter_value as u64))
    }
}

#[inline(always)]
/// 获取 cpu/架构下的定时器计数器的值
fn read_cntpct() -> GenericTimerCounterValue {
    // Prevent that the counter is read ahead of time due to out-of-order execution.
    barrier::isb(barrier::SY);
    let cnt = CNTPCT_EL0.get();

    GenericTimerCounterValue(cnt)
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// timer的分辨率对应的Duration
pub fn resolution() -> Duration {
    Duration::from(GenericTimerCounterValue(1))
}

/// 获取从上电开始到现在的运行时间
/// 包括固件和引导程序消耗的时间
pub fn uptime() -> Duration {
    read_cntpct().into()
}

/// 原地忙等指定的duration
pub fn spin_for(duration: Duration) {
    let curr_counter_value = read_cntpct();

    let counter_value_delta: GenericTimerCounterValue = match duration.try_into() {
        Err(msg) => {
            warn!("spin_for: {}. Skipping", msg);
            return;
        }
        Ok(val) => val,
    };
    let counter_value_target = curr_counter_value + counter_value_delta;

    // 忙等待, 直至 CNTPCT_EL0 的值 大于等于 要延迟到的时间点
    // 这里没有直接用 read_cntpct(),避过了isb 指令
    while GenericTimerCounterValue(CNTPCT_EL0.get()) < counter_value_target {}
}
