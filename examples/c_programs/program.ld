/* Linker script for RISC-V programs */

OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    /* Define memory regions */
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
    /* Code section */
    .text : {
        *(.text .text.*)
    } > RAM

    /* Read-only data section */
    .rodata : {
        *(.rodata .rodata.*)
    } > RAM

    /* Read-write data section */
    .data : {
        *(.data .data.*)
    } > RAM

    /* BSS section (uninitialized data) */
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } > RAM

    /* Stack pointer initialization */
    . = ALIGN(16);
    _stack_top = .;
    . = . + 0x1000; /* 4KB stack */
    _stack_bottom = .;
} 