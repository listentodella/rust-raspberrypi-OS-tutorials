// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2020-2023 Andre Richter <andre.o.richter@gmail.com>

//! Synchronization primitives.
//!
//! # Resources
//!
//!   - <https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html>
//!   - <https://stackoverflow.com/questions/59428096/understanding-the-send-trait>
//!   - <https://doc.rust-lang.org/std/cell/index.html>

use core::cell::UnsafeCell;

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// 同步接口
pub mod interface {

    /// 任何实现了该trait的对象, 保证在提供的闭包期间, 对Mutex包裹的数据进行独占访问.
    pub trait Mutex {
        /// 被mutex包裹的数据的类型, 即关联类型
        type Data;

        /// mutex被锁定, 并向闭包提供临时可变访问被包裹的数据的权限.
        fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
    }
}

/// 仅用于教学目的的伪锁.
///
/// 相对于真实的Mutex实现, 该伪锁并未对包含的数据进行并发访问的保护. 这一部分保留用于后续的课程.
///
/// 该锁的安全使用是有条件的,比如, 只要kernel是单线程的, 即单核且禁用中断的情况下, 才能安全使用.
pub struct NullLock<T>
where
    T: ?Sized,
{
    data: UnsafeCell<T>,
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// 当前可以认为NullLock是Send和Sync的,
/// 实现这两个trait可以允许在线程间安全的传递该锁.
unsafe impl<T> Send for NullLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for NullLock<T> where T: ?Sized + Send {}

impl<T> NullLock<T> {
    /// 创建一个 `NullLock`, 将要包裹的数据作为参数传入.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------

/// 为 `NullLock<T>` 实现 `Mutex` trait, 这样借助`Mutex`, 可以独占访问内部的数据
impl<T> interface::Mutex for NullLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        // In a real lock, there would be code encapsulating this line that ensures that this
        // mutable reference will ever only be given out once at a time.
        // 如果是真实的锁, 则会有一段代码来确保只会给出一次可变引用.
        let data = unsafe { &mut *self.data.get() };

        // 通过闭包访问该可变引用
        f(data)
    }
}
