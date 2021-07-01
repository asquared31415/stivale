use core::marker::PhantomData;

use super::header::StivaleSmpHeaderTagFlags;

#[repr(C, packed)]
pub struct StivaleTagHeader {
    pub identifier: u64,
    pub next: u64,
}

/// If the framebuffer tag was requested through the framebuffer tag header and its supported by the stivale
/// bootloader, this tag is returned to the kernel. This tag provides an interface to the framebuffer.
#[repr(C, packed)]
pub struct StivaleFramebufferTag {
    pub header: StivaleTagHeader,
    /// The address of the framebuffer.
    pub framebuffer_addr: u64,
    /// The total width of the framebuffer in pixels.
    pub framebuffer_width: u16,
    /// The total height of the framebuffer in pixels.
    pub framebuffer_height: u16,
    /// The pitch of the framebuffer in bytes.
    pub framebuffer_pitch: u16,
    /// The amount of bytes-per pixel.
    pub framebuffer_bpp: u16,
    /// Memory model of the framebuffer. If set to one, its RGB and all other values
    /// are undefined.
    pub memory_model: u8,
    /// Size of the red mask in RGB.
    pub red_mask_size: u8,
    /// Shift of the red mask in RGB.
    pub red_mask_shift: u8,
    /// Size of the green mask in RGB.
    pub green_mask_size: u8,
    /// Shift of the green mask in RGB.
    pub green_mask_shift: u8,
    /// Size of the blue mask in RGB.
    pub blue_mask_size: u8,
    /// Shift of the blue mask in RGB.
    pub blue_mask_shift: u8,
}

impl StivaleFramebufferTag {
    /// Returns the size of the framebuffer.
    pub fn size(&self) -> usize {
        self.framebuffer_pitch as usize
            * self.framebuffer_height as usize
            * (self.framebuffer_bpp as usize / 8)
    }
}

/// If the terminal tag was requested through the terminal tag header and its supported by the stivale
/// bootloader, this tag is returned to the kernel. This tag provides an interface to the stivale terminal.
#[repr(C, packed)]
pub struct StivaleTerminalTag {
    pub header: StivaleTagHeader,
    pub flags: u32,
    /// The amount of columns in the stivale terminal setup by the bootloader.
    pub cols: u16,
    /// The amount of rows in the stivale terminal setup by the bootloader.
    pub rows: u16,
    /// The virtual address of the `term_write` function, which is used to write to the stivale terminal. For
    /// a more safer way use the [StivaleTerminalTag::term_write]
    pub term_write_addr: u64,
}

impl StivaleTerminalTag {
    /// Returns the terminal write function provided by the terminal stivale tag. This function
    /// returns the transmuted function for you to simplify the process of passing the string as a raw pointer
    /// and passing the string length.
    ///
    /// ## Example
    /// ```rust,no_run
    /// let terminal_tag = stivale_struct.terminal().expect("Terminal tag was provided by the stivale2 bootloader");
    /// let term_write = terminal_tag.term_write();
    ///
    /// term_write("Hello, Stivale!");
    /// term_write("Hello, Rust!")
    /// ```
    ///
    /// ## Safety
    /// This function is **not** thread safe.
    pub fn term_write(&self) -> impl Fn(&str) {
        let __fn_ptr = self.term_write_addr as *const ();
        let __term_func =
            unsafe { core::mem::transmute::<*const (), extern "C" fn(*const i8, u64)>(__fn_ptr) };

        move |txt| {
            __term_func(txt.as_ptr() as *const i8, txt.len() as u64);
        }
    }
}

/// This tag is used to get the location of the ACPI RSDP structure in memory.
#[repr(C, packed)]
pub struct StivaleRsdpTag {
    pub header: StivaleTagHeader,
    /// Pointer to the ACPI RSDP structure.
    pub rsdp: u64,
}

/// The type of a memory map entry. The entries are guaranteed to be sorted by base address,
/// lowest to highest.
///
/// ## Alignment
/// Usable and bootloader reclaimable entries are guaranteed to be 4096 byte aligned for both
/// base and length. Usable and bootloader reclaimable entries are **guaranteed** not to overlap with
/// any other entry.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StivaleMemoryMapEntryType {
    /// Usable memory.
    Usable = 1,
    /// Memory reserved by the system.
    Reserved = 2,
    /// ACPI memory that can be reclaimed.
    AcpiReclaimable = 3,
    /// ACPI memory that cannot be reclaimed.
    AcpiNvs = 4,
    /// Memory marked as defective (bad RAM).
    BadMemory = 5,
    /// Memory used by the bootloader that can be reclaimed after it's not being used anymore.
    BootloaderReclaimable = 0x1000,
    /// Memory containing the kernel and any modules.
    Kernel = 0x1001,
    /// Memory containing the framebuffer.
    Framebuffer = 0x1002,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct StivaleMemoryMapEntry {
    /// Physical address of base of the memory section.
    pub base: u64,
    /// Length of this memory section.
    pub length: u64,
    /// The type of this memory map entry.
    pub entry_type: StivaleMemoryMapEntryType,

    padding: u32,
}

impl StivaleMemoryMapEntry {
    /// Returns the end address of this memory region.
    #[inline]
    pub fn end_address(&self) -> u64 {
        self.base + self.length
    }

    /// Returns the entry type of this memory region. External function is required
    /// as reference the entry_type packed field is not aligned.
    #[inline]
    pub fn entry_type(&self) -> StivaleMemoryMapEntryType {
        self.entry_type
    }
}

#[repr(C, packed)]
pub struct StivaleMemoryMapTag {
    pub header: StivaleTagHeader,
    /// Total length of the memory map entries.
    pub entries_len: u64,
    /// Pointer to the memory map entries.
    pub entry_array: [StivaleMemoryMapEntry; 0],
}

impl StivaleMemoryMapTag {
    /// Return's memory map entries pointer as a rust slice.
    pub fn as_slice(&self) -> &[StivaleMemoryMapEntry] {
        unsafe { core::slice::from_raw_parts(self.entry_array.as_ptr(), self.entries_len as usize) }
    }

    /// Returns an iterator over all the memory regions.
    pub fn iter(&self) -> StivaleMemoryMapIter {
        StivaleMemoryMapIter {
            sref: self,
            current: 0x00,
            phantom: PhantomData::default(),
        }
    }
}

/// Iterator over all the memory regions provided by the stivale bootloader.
#[derive(Clone)]
pub struct StivaleMemoryMapIter<'a> {
    /// A reference to the stivale memory map tag.
    sref: &'a StivaleMemoryMapTag,
    /// The index of the memory map entry that we are about to index.
    current: u64,
    phantom: PhantomData<&'a StivaleMemoryMapEntry>,
}

impl<'a> Iterator for StivaleMemoryMapIter<'a> {
    type Item = &'a StivaleMemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.sref.entries_len {
            let entry = &self.sref.as_slice()[self.current as usize];
            self.current += 1;

            Some(entry)
        } else {
            None
        }
    }
}

/// This tag is used to get the current UNIX epoch, as per RTC.
#[repr(C, packed)]
pub struct StivaleEpochTag {
    pub header: StivaleTagHeader,
    /// UNIX epoch at boot, which is read from system RTC.
    pub epoch: u64,
}

bitflags::bitflags! {
    /// Bitfield representing the firmware and boot flags passed by the bootloader.
    pub struct StivaleFirmwareTagFlags: u64 {
        /// The kernel was booted in UEFI mode.
        const UEFI = 0x00;
        /// The kernel was booted in a legacy BIOS mode.
        const BIOS = 0x01;
    }
}

/// This tag is used to get the info about the firmware.
#[repr(C, packed)]
pub struct StivaleFirmwareTag {
    pub header: StivaleTagHeader,
    /// Flags telling about the firmware and boot flags passed by the bootloader.
    pub flags: StivaleFirmwareTagFlags,
}

/// This tag is used to get a pointer to the EFI system table if available.
#[repr(C, packed)]
pub struct StivaleEfiSystemTableTag {
    pub header: StivaleTagHeader,
    /// Address of the EFI system table.
    pub system_table_addr: u64,
}

/// This tag is used to get the kernel with a pointer to a copy the raw executable
/// file of the kernel that the bootloader loaded.
#[repr(C, packed)]
pub struct StivaleKernelFileTag {
    pub header: StivaleTagHeader,
    /// Address of the raw kernel file.
    pub kernel_file_addr: u64,
}

/// This tag is used to get the slide that the bootloader applied over the kernel's load
/// address as a positive offset.
#[repr(C, packed)]
pub struct StivaleKernelSlideTag {
    pub header: StivaleTagHeader,
    /// The kernel slide. See structure-level documentation for more information.
    pub kernel_slide: u64,
}

/// This tag is used to get the kernel the command line string that was passed to it by
/// the bootloader.
#[repr(C, packed)]
pub struct StivaleCommandLineTag {
    pub header: StivaleTagHeader,
    /// Pointer to a null-terminated cmdline.
    pub command_line: u64,
}

/// This tag is used to get the EDID information as acquired by the firmware.
#[repr(C, packed)]
pub struct StivaleEdidInfoTag {
    pub header: StivaleTagHeader,
    /// Length of the EDID information array.
    pub edid_len: u64,
    /// Pointer to the EDID information array.
    pub info_array: [u8; 0],
}

impl StivaleEdidInfoTag {
    /// Return's the EDID information pointer as a rust slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.info_array.as_ptr(), self.edid_len as usize) }
    }
}

/// This tag exists if MTRR write-combining for the framebuffer was requested and successfully enabled. See
/// the documentation of [crate::v2::header::StivaleMtrrHeaderTag] for more information.
///
/// ## Legacy
/// This tag is deprecated and considered legacy. Use is discouraged and it may not be supported on newer bootloaders.
#[deprecated(
    note = "This tag is deprecated and considered legacy. Use is discouraged and it may not be supported on newer bootloaders."
)]
#[repr(C, packed)]
pub struct StivaleMtrrTag {
    pub header: StivaleTagHeader,
}

/// Structure representing a module, containing the information of a module that
/// the bootloader loaded alongside the kernel.
#[repr(C, packed)]
pub struct StivaleModule {
    /// Address where this module has been loaded.
    pub start: u64,
    /// End address of this module.
    pub end: u64,
    /// ASCII 0-terminated string passed to the module as specified in
    /// the config file.
    pub string: [u8; 128],
}

impl StivaleModule {
    /// Returns the size of this module.
    #[inline]
    pub fn size(&self) -> u64 {
        self.end - self.start
    }

    /// Returns the ASCII 0-terminated string passed to the module as specified in the config file
    /// as a rust string.
    #[inline]
    pub fn as_str(&self) -> &str {
        super::utils::string_from_slice(&self.string)
    }
}

/// Iterator over all the modules that were loaded.
#[derive(Clone)]
pub struct StivaleModuleIter<'a> {
    /// A reference to the stivale module tag.
    sref: &'a StivaleModuleTag,
    /// The index of the module entry that we are about to index.
    current: u64,
    phantom: PhantomData<&'a StivaleModule>,
}

impl<'a> Iterator for StivaleModuleIter<'a> {
    type Item = &'a StivaleModule;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.sref.module_len {
            let entry = &self.sref.as_slice()[self.current as usize];
            self.current += 1;

            Some(entry)
        } else {
            None
        }
    }
}

/// This tag is used to get the modules that the bootloader loaded alongside the kernel, if any.
#[repr(C, packed)]
pub struct StivaleModuleTag {
    pub header: StivaleTagHeader,
    /// Length of the modules array.
    pub module_len: u64,
    /// Pointer to the modules array.
    pub modules_array: [StivaleModule; 0],
}

impl StivaleModuleTag {
    /// Returns an iterator over all the modules that were loaded.
    pub fn iter(&self) -> StivaleModuleIter {
        StivaleModuleIter {
            sref: self,
            current: 0,
            phantom: PhantomData::default(),
        }
    }

    /// Return's the modules array pointer as a rust slice.
    pub fn as_slice(&self) -> &[StivaleModule] {
        unsafe {
            core::slice::from_raw_parts(self.modules_array.as_ptr(), self.module_len as usize)
        }
    }
}

/// This tag is used to get the location of the SMBIOS entry points in memory.
#[repr(C, packed)]
pub struct StivaleSmbiosTag {
    pub header: StivaleTagHeader,
    /// Stivale specification says that the flags in this tag are for future use
    /// and currently should be unused and **must** be 0.
    pub flags: u64,
    /// 32-bit SMBIOS entry point address. Set to 0 if unavailable.
    pub smbios_entry_32: u64,
    /// 64-bit SMBIOS entry point address. Set to 0 if unavailable.
    pub smbios_entry_64: u64,
}

/// SMP imformation structure.
#[repr(C, packed)]
pub struct StivaleSmpInfo {
    /// ACPI Processor UID as specified by MADT.
    pub acpi_processor_uid: u32,
    /// LAPIC ID as specified by MADT.
    pub lapic_id: u32,
    /// The stack that will be loaded in ESP/RSP once the goto_address field is loaded.
    /// This MUST point to a valid stack of at least 256 bytes in size, and 16-byte aligned.
    /// target_stack is an unused field for the struct describing the BSP.
    pub target_stack: u64,
    /// This field is polled by the started APs until the kernel on another CPU performs an
    /// atomic write to this field. When that happens, bootloader code will load up ESP/RSP with the stack
    /// value as specified in target_stack. It will then proceed to load a pointer to this very structure into
    /// either register RDI for 64-bit or on the stack for 32-bit, then, goto_address is called (a bogus return
    /// address is pushed onto the stack) and execution is handed off.
    /// The CPU state will be the same as describedin "kernel entry machine state", with the exception
    /// of ESP/RSP and RDI/stack arg being set up as above. goto_address is an unused field for the
    /// struct describing the BSP.
    pub goto_address: u64,
    /// This field is polled by the started APs until the kernel on another CPU performs an
    /// atomic write to this field. When that happens, bootloader code will
    /// load up ESP/RSP with the stack value as specified in target_stack.
    /// It will then proceed to load a pointer to this very structure into either register
    /// RDI for 64-bit or on the stack for 32-bit, then, goto_address is called (a bogus return
    /// address is pushed onto the stack) and execution is handed off.
    ///
    /// The CPU state will be the same as described in "kernel entry machine state", with the exception
    /// of ESP/RSP and RDI/stack arg being set up as above. goto_address is an unused field for the
    /// struct describing the BSP.
    pub extra: u64,
}

#[repr(C, packed)]
pub struct StivaleSmpTag {
    pub header: StivaleTagHeader,
    pub flags: StivaleSmpHeaderTagFlags,
    /// LAPIC ID of the BSP (bootstrap processor).
    pub bsp_lapic_id: u32,
    /// Stivale specification says that this field is reserved for future use.
    pub unused: u32,
    /// The total number of logical CPUs (including BSP).
    pub cpu_count: u64,
    /// Pointer to the SMP info array (including BSP).
    pub smp_info_array: [StivaleSmpInfo; 0],
}

impl StivaleSmpTag {
    /// Return's the SMP info array pointer as a rust slice.
    pub fn as_slice(&self) -> &[StivaleSmpInfo] {
        unsafe {
            core::slice::from_raw_parts(self.smp_info_array.as_ptr(), self.cpu_count as usize)
        }
    }
}

/// This tag reports that the kernel has been booted via PXE, and reports the server ip that
/// it was booted from.
#[repr(C, packed)]
pub struct StivalePxeInfoTag {
    pub header: StivaleTagHeader,
    /// Server IP in network byte order.
    pub server_ip: u32,
}

/// This tag reports that there is a memory mapped UART port and its address.
#[repr(C, packed)]
pub struct StivaleUartTag {
    pub header: StivaleTagHeader,
    /// The address of the UART port.
    pub address: u64,
}

/// This tag describes a device tree blob for the platform.
#[repr(C, packed)]
pub struct StivaleDeviceTreeTag {
    pub header: StivaleTagHeader,
    /// The address of the device tree blob.
    pub address: u64,
    /// The size of the device tree blob.
    pub size: u64,
}

/// This tag describes the high physical memory location.
#[repr(C, packed)]
pub struct StivaleVMap {
    pub header: StivaleTagHeader,
    /// VMAP_HIGH, where the physical memory is mapped in the higher half.
    pub address: u64,
}