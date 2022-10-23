// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2020-2022 Andre Richter <andre.o.richter@gmail.com>

//! GICv2 Driver - ARM Generic Interrupt Controller v2.
//!
//! The following is a collection of excerpts with useful information from
//!   - `Programmer's Guide for ARMv8-A`
//!   - `ARM Generic Interrupt Controller Architecture Specification`
//!
//! # Programmer's Guide - 10.6.1 Configuration
//!
//! The GIC is accessed as a memory-mapped peripheral.
//!
//! All cores can access the common Distributor, but the CPU interface is banked, that is, each core
//! uses the same address to access its own private CPU interface.
//!
//! It is not possible for a core to access the CPU interface of another core.
//!
//! # Architecture Specification - 10.6.2 Initialization
//!
//! Both the Distributor and the CPU interfaces are disabled at reset. The GIC must be initialized
//! after reset before it can deliver interrupts to the core.
//!
//! In the Distributor, software must configure the priority, target, security and enable individual
//! interrupts. The Distributor must subsequently be enabled through its control register
//! (GICD_CTLR). For each CPU interface, software must program the priority mask and preemption
//! settings.
//!
//! Each CPU interface block itself must be enabled through its control register (GICD_CTLR). This
//! prepares the GIC to deliver interrupts to the core.
//!
//! Before interrupts are expected in the core, software prepares the core to take interrupts by
//! setting a valid interrupt vector in the vector table, and clearing interrupt mask bits in
//! PSTATE, and setting the routing controls.
//!
//! The entire interrupt mechanism in the system can be disabled by disabling the Distributor.
//! Interrupt delivery to an individual core can be disabled by disabling its CPU interface.
//! Individual interrupts can also be disabled (or enabled) in the distributor.
//!
//! For an interrupt to reach the core, the individual interrupt, Distributor and CPU interface must
//! all be enabled. The interrupt also needs to be of sufficient priority, that is, higher than the
//! core's priority mask.
//!
//! # Architecture Specification - 1.4.2 Interrupt types
//!
//! - Peripheral interrupt
//!     - Private Peripheral Interrupt (PPI)
//!         - This is a peripheral interrupt that is specific to a single processor.
//!     - Shared Peripheral Interrupt (SPI)
//!         - This is a peripheral interrupt that the Distributor can route to any of a specified
//!           combination of processors.
//!
//! - Software-generated interrupt (SGI)
//!     - This is an interrupt generated by software writing to a GICD_SGIR register in the GIC. The
//!       system uses SGIs for interprocessor communication.
//!     - An SGI has edge-triggered properties. The software triggering of the interrupt is
//!       equivalent to the edge transition of the interrupt request signal.
//!     - When an SGI occurs in a multiprocessor implementation, the CPUID field in the Interrupt
//!       Acknowledge Register, GICC_IAR, or the Aliased Interrupt Acknowledge Register, GICC_AIAR,
//!       identifies the processor that requested the interrupt.
//!
//! # Architecture Specification - 2.2.1 Interrupt IDs
//!
//! Interrupts from sources are identified using ID numbers. Each CPU interface can see up to 1020
//! interrupts. The banking of SPIs and PPIs increases the total number of interrupts supported by
//! the Distributor.
//!
//! The GIC assigns interrupt ID numbers ID0-ID1019 as follows:
//!   - Interrupt numbers 32..1019 are used for SPIs.
//!   - Interrupt numbers 0..31 are used for interrupts that are private to a CPU interface. These
//!     interrupts are banked in the Distributor.
//!       - A banked interrupt is one where the Distributor can have multiple interrupts with the
//!         same ID. A banked interrupt is identified uniquely by its ID number and its associated
//!         CPU interface number. Of the banked interrupt IDs:
//!           - 00..15 SGIs
//!           - 16..31 PPIs

mod gicc;
mod gicd;

use crate::{
    bsp::{self, device_driver::common::BoundedUsize},
    cpu, driver, exception, synchronization,
    synchronization::InitStateLock,
};

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

type HandlerTable = [Option<exception::asynchronous::IRQHandlerDescriptor<IRQNumber>>;
    IRQNumber::MAX_INCLUSIVE + 1];

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Used for the associated type of trait [`exception::asynchronous::interface::IRQManager`].
pub type IRQNumber = BoundedUsize<{ GICv2::MAX_IRQ_NUMBER }>;

/// Representation of the GIC.
pub struct GICv2 {
    /// The Distributor.
    gicd: gicd::GICD,

    /// The CPU Interface.
    gicc: gicc::GICC,

    /// Stores registered IRQ handlers. Writable only during kernel init. RO afterwards.
    handler_table: InitStateLock<HandlerTable>,
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl GICv2 {
    const MAX_IRQ_NUMBER: usize = 300; // Normally 1019, but keep it lower to save some space.

    pub const COMPATIBLE: &'static str = "GICv2 (ARM Generic Interrupt Controller v2)";

    /// Create an instance.
    ///
    /// # Safety
    ///
    /// - The user must ensure to provide a correct MMIO start address.
    pub const unsafe fn new(gicd_mmio_start_addr: usize, gicc_mmio_start_addr: usize) -> Self {
        Self {
            gicd: gicd::GICD::new(gicd_mmio_start_addr),
            gicc: gicc::GICC::new(gicc_mmio_start_addr),
            handler_table: InitStateLock::new([None; IRQNumber::MAX_INCLUSIVE + 1]),
        }
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------
use synchronization::interface::ReadWriteEx;

impl driver::interface::DeviceDriver for GICv2 {
    type IRQNumberType = IRQNumber;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    unsafe fn init(&self) -> Result<(), &'static str> {
        if bsp::cpu::BOOT_CORE_ID == cpu::smp::core_id() {
            self.gicd.boot_core_init();
        }

        self.gicc.priority_accept_all();
        self.gicc.enable();

        Ok(())
    }
}

impl exception::asynchronous::interface::IRQManager for GICv2 {
    type IRQNumberType = IRQNumber;

    fn register_handler(
        &self,
        irq_handler_descriptor: exception::asynchronous::IRQHandlerDescriptor<Self::IRQNumberType>,
    ) -> Result<(), &'static str> {
        self.handler_table.write(|table| {
            let irq_number = irq_handler_descriptor.number().get();

            if table[irq_number].is_some() {
                return Err("IRQ handler already registered");
            }

            table[irq_number] = Some(irq_handler_descriptor);

            Ok(())
        })
    }

    fn enable(&self, irq_number: &Self::IRQNumberType) {
        self.gicd.enable(irq_number);
    }

    fn handle_pending_irqs<'irq_context>(
        &'irq_context self,
        ic: &exception::asynchronous::IRQContext<'irq_context>,
    ) {
        // Extract the highest priority pending IRQ number from the Interrupt Acknowledge Register
        // (IAR).
        let irq_number = self.gicc.pending_irq_number(ic);

        // Guard against spurious interrupts.
        if irq_number > GICv2::MAX_IRQ_NUMBER {
            return;
        }

        // Call the IRQ handler. Panic if there is none.
        self.handler_table.read(|table| {
            match table[irq_number] {
                None => panic!("No handler registered for IRQ {}", irq_number),
                Some(descriptor) => {
                    // Call the IRQ handler. Panics on failure.
                    descriptor.handler().handle().expect("Error handling IRQ");
                }
            }
        });

        // Signal completion of handling.
        self.gicc.mark_comleted(irq_number as u32, ic);
    }

    fn print_handler(&self) {
        use crate::info;

        info!("      Peripheral handler:");

        self.handler_table.read(|table| {
            for (i, opt) in table.iter().skip(32).enumerate() {
                if let Some(handler) = opt {
                    info!("            {: >3}. {}", i + 32, handler.name());
                }
            }
        });
    }
}
