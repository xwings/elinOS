/* Bootloader linker script for RISC-V 64-bit */
MEMORY {
    /* Bootloader loads at standard kernel address where OpenSBI expects it */
    BOOTLOADER (rwx) : ORIGIN = 0x80200000, LENGTH = 2M
    /* Kernel space starts after bootloader */
    KERNEL_SPACE (rwx) : ORIGIN = 0x80400000, LENGTH = 2M
    /* RAM for heap and stacks starts at 0x80600000 */
    RAM (rwx) : ORIGIN = 0x80600000, LENGTH = 58M
}

PHDRS {
    text PT_LOAD FLAGS(5);  /* R-X */
    rodata PT_LOAD FLAGS(4);  /* R-- */
    data PT_LOAD FLAGS(6);  /* RW- */
    bss PT_LOAD FLAGS(6);  /* RW- */
    ram PT_LOAD FLAGS(6);  /* RW- */
}

/* Stack configuration for bootloader */
PROVIDE(_bootloader_stack_size = 64K);  /* 64KB for bootloader stack */
PROVIDE(_stack_size = 2M);  /* 2MB per hart stack */
PROVIDE(_max_hart_id = 4);  /* Support up to 4 harts */
PROVIDE(_hart_stack_size = _stack_size);
PROVIDE(_heap_size = 32M);  /* Default 32MB for heap, can be adjusted */

OUTPUT_ARCH(riscv)
OUTPUT_FORMAT("elf64-littleriscv")
ENTRY(_start)

SECTIONS {
    /* Bootloader starts at 0x80200000 where OpenSBI expects the kernel */
    . = 0x80200000;

    .text : {
        __text_start = .;
        __bootloader_text_start = .;
        KEEP(*(.text.boot))
        KEEP(*(.text.kernel))
        *(.text .text.*)
        __bootloader_text_end = .;
        __text_end = .;
    } :text

    .rodata : {
        __rodata_start = .;
        __bootloader_rodata_start = .;
        *(.rodata .rodata.*)
        __bootloader_rodata_end = .;
        __rodata_end = .;
    } :rodata

    .data : {
        __data_start = .;
        __bootloader_data_start = .;
        *(.data .data.*)
        __bootloader_data_end = .;
        __data_end = .;
    } :data

    .bss : {
        __bss_start = .;
        __bootloader_bss_start = .;
        *(.bss .bss.*)
        __bootloader_bss_end = .;
        __bss_end = .;
    } :bss

    /* Move to RAM region for heap and stack */
    . = 0x80600000;

    /* Heap starts at beginning of RAM - size determined at runtime */
    _heap_start = .;
    .heap : {
        . = . + _heap_size;
    } >RAM AT>RAM :ram

    /* Stack space for each hart */
    . = ALIGN(4096);
    _stack_start = .;
    __stack_bottom = .;
    .stack : {
        . = . + (_hart_stack_size * (_max_hart_id + 1));
    } >RAM AT>RAM :ram
    _stack_end = .;
    __stack_top = .;

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }

    /* End of bootloader */
    . = ALIGN(4096);
    __bootloader_end = .;
    end = .;
}

/* Define symbols for memory management */
PROVIDE(_kernel_end = end);
PROVIDE(_heap_end = _heap_start + _heap_size);
PROVIDE(_stack_top = _stack_end);

/* Runtime memory detection will override these defaults */
PROVIDE(_runtime_heap_start = 0);
PROVIDE(_runtime_heap_end = 0);
PROVIDE(_runtime_total_memory = 0);