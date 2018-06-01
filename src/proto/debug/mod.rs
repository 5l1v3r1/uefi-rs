//! Provides support for the UEFI debugging protocol.
//!
//! This protocol is designed to allow debuggers to query the state of the firmware,
//! as well as set up callbacks for various events.
//!
//! It also defines a Debugport protocol for debugging over serial devices.
//!
//! An example UEFI debugger is Intel's [UDK Debugger Tool][udk].
//!
//! [udk]: https://firmware.intel.com/develop/intel-uefi-tools-and-utilities/intel-uefi-development-kit-debugger-tool

#[repr(C)]
pub struct DebugSupport {
    isa: ProcessorArch,
}

impl DebugSupport {
    /// Returns the processor architecture of the running CPU.
    pub fn arch(&self) -> ProcessorArch {
        self.isa
    }
}

/// The instruction set architecture of the running processor.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum ProcessorArch {
    /// 32-bit x86 PC
    X86_32 = 0x014C,
    /// 64-bit x86 PC
    X86_64 = 0x8664,
    /// Intel Itanium
    Itanium = 0x200,
    /// UEFI Interpreter bytecode
    EBC = 0x0EBC,
    /// ARM Thumb / Mixed
    Arm = 0x01C2,
    /// ARM 64-bit
    AArch64 = 0xAA64,
    /// RISC-V 32-bit
    RiscV32 = 0x5032,
    /// RISC-V 64-bit
    RiscV64 = 0x5064,
    /// RISC-V 128-bit
    RiscV128 = 0x5128,
}

impl_proto! {
    protocol DebugSupport {
        GUID = 0x2755590C, 0x6F3C, 0x42FA, [0x9E, 0xA4, 0xA3, 0xBA, 0x54, 0x3C, 0xDA, 0x25];
    }
}
