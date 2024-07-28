// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2023 Andre Richter <andre.o.richter@gmail.com>

//! Driver support.

use crate::{
    println,
    // 此时的Mutex依旧是有条件的安全
    synchronization::{interface::Mutex, NullLock},
};

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

const NUM_DRIVERS: usize = 5;

// 设备驱动管理器内部结构体
// 这里为了简化, 只用了一个固定大小的数组来存放多种设备描述符
// 使用next_index来记录下一个数组空闲的位置
struct DriverManagerInner {
    next_index: usize,
    /// 设备驱动描述符数组
    /// 用Option封装, 说明该数组有的元素可能没有设备驱动描述符
    descriptors: [Option<DeviceDriverDescriptor>; NUM_DRIVERS],
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Driver interfaces.
pub mod interface {
    /// 设备驱动trait
    pub trait DeviceDriver {
        /// 返回兼容性字符串，用于识别驱动程序
        fn compatible(&self) -> &'static str;

        /// 用于kernel初始化驱动程序
        ///
        /// # Safety
        ///
        /// - 初始化期间, 驱动程序可能对系统产生影响.
        unsafe fn init(&self) -> Result<(), &'static str> {
            Ok(())
        }
    }
}

/// 其实只是个别名, 实际上是unsafe fn() -> Result<(), &'static str>
/// 如果成功, 将返回(); 失败, 返回一个字符串描述错误原因
pub type DeviceDriverPostInitCallback = unsafe fn() -> Result<(), &'static str>;

/// 用于描述设备驱动的结构体
#[derive(Copy, Clone)]
pub struct DeviceDriverDescriptor {
    // 设备驱动,要求实现DeviceDriver trait
    // 以及Sync trait, 用于跨线程访问
    device_driver: &'static (dyn interface::DeviceDriver + Sync),
    // 用Option封装, 说明可能没有 DeviceDriverPostInitCallback
    post_init_callback: Option<DeviceDriverPostInitCallback>,
}

/// 提供设备驱动管理函数
/// 目前还是通过伪锁提供独占保护, 只要单核且中断禁止, 就能保证安全.
pub struct DriverManager {
    inner: NullLock<DriverManagerInner>,
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

/// 全局变量, 用于管理设备驱动
static DRIVER_MANAGER: DriverManager = DriverManager::new();

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

impl DriverManagerInner {
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            next_index: 0,
            descriptors: [None; NUM_DRIVERS],
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl DeviceDriverDescriptor {
    /// Create an instance.
    pub fn new(
        device_driver: &'static (dyn interface::DeviceDriver + Sync),
        post_init_callback: Option<DeviceDriverPostInitCallback>,
    ) -> Self {
        Self {
            device_driver,
            post_init_callback,
        }
    }
}

/// 返回一个全局的DriverManager实例的引用
pub fn driver_manager() -> &'static DriverManager {
    &DRIVER_MANAGER
}

impl DriverManager {
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            // 使用NUllLock封装, 提供一定程度的安全访问
            inner: NullLock::new(DriverManagerInner::new()),
        }
    }

    /// 向kernel注册一个设备驱动
    pub fn register_driver(&self, descriptor: DeviceDriverDescriptor) {
        self.inner.lock(|inner| {
            inner.descriptors[inner.next_index] = Some(descriptor);
            inner.next_index += 1;
        })
    }

    /// 辅助函数, 用于遍历已注册的设备驱动, 并调用闭包
    fn for_each_descriptor<'a>(&'a self, f: impl FnMut(&'a DeviceDriverDescriptor)) {
        self.inner.lock(|inner| {
            inner
                .descriptors
                .iter()
                // 过滤掉None元素
                .filter_map(|x| x.as_ref())
                .for_each(f)
        })
    }

    /// 初始化所有已注册的设备驱动
    ///
    /// # Safety
    ///
    /// - During init, drivers might do stuff with system-wide impact.
    pub unsafe fn init_drivers(&self) {
        self.for_each_descriptor(|descriptor| {
            // 1. Initialize driver.
            // 调用驱动的init函数
            // 如果device_driver.init()失败, 则panic
            if let Err(x) = descriptor.device_driver.init() {
                panic!(
                    "Error initializing driver: {}: {}",
                    descriptor.device_driver.compatible(),
                    x
                );
            }

            // 2. Call corresponding post init callback.
            // 如果有post_init_callback, 则调用之
            if let Some(callback) = &descriptor.post_init_callback {
                // 该callback如果失败, 就会返回一个x:&'static str描述错误原因
                // 这里如果失败, 就将原因x通过panic输出
                if let Err(x) = callback() {
                    panic!(
                        "Error during driver post-init callback: {}: {}",
                        descriptor.device_driver.compatible(),
                        x
                    );
                }
            }
        });
    }

    /// 枚举所有已注册的设备驱动
    /// 实际只是打印它们的兼容性字符串
    pub fn enumerate(&self) {
        let mut i: usize = 1;
        self.for_each_descriptor(|descriptor| {
            println!("      {}. {}", i, descriptor.device_driver.compatible());

            i += 1;
        });
    }
}
