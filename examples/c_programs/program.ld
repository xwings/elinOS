/* Linker script for RISC-V programs */

OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    /* Define memory regions - start at 4-byte aligned address */
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
    /* Code section - ensure 4-byte alignment */
    .text 0x80000000 : ALIGN(4) {
        KEEP(*(.text.start))
        *(.text .text.*)
    } > RAM

    /* Read-only data section */
    .rodata : ALIGN(4) {
        *(.rodata .rodata.*)
    } > RAM

    /* Read-write data section */
    .data : ALIGN(4) {
        *(.data .data.*)
    } > RAM

    /* BSS section (uninitialized data) */
    .bss : ALIGN(4) {
        *(.bss .bss.*)
        *(COMMON)
    } > RAM

    /* Stack pointer initialization */
    . = ALIGN(16);
    _stack_top = .;
    . = . + 0x1000; /* 4KB stack */
    _stack_bottom = .;
} 