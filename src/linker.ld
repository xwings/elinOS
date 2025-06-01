/* Memory layout for RISC-V 64-bit */
MEMORY {
    /* 2MB for kernel code and data */
    KERNEL (rwx) : ORIGIN = 0x80200000, LENGTH = 2M
    /* 126MB for heap and stack */
    RAM (rwx) : ORIGIN = 0x80400000, LENGTH = 126M
}

/* Define output sections */
PHDRS {
    text PT_LOAD FLAGS(5);  /* R-X */
    rodata PT_LOAD FLAGS(4);  /* R-- */
    data PT_LOAD FLAGS(6);  /* RW- */
    bss PT_LOAD FLAGS(6);  /* RW- */
    ram PT_LOAD FLAGS(6);  /* RW- */
}

/* Stack configuration */
PROVIDE(_stack_size = 2M);  /* 2MB per hart stack */
PROVIDE(_max_hart_id = 4);  /* Support up to 4 harts */
PROVIDE(_hart_stack_size = _stack_size);

/* Heap configuration */
PROVIDE(_heap_size = 64M);  /* 64MB for heap */

OUTPUT_ARCH(riscv)
OUTPUT_FORMAT("elf64-littleriscv")
ENTRY(_start)

SECTIONS {
    /* Kernel code and data at 0x80200000 */
    . = 0x80200000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    } :text

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
    } :bss

    /* Move to RAM region for heap and stack */
    . = 0x80400000;

    /* Heap starts at beginning of RAM */
    _heap_start = .;
    .heap : {
        . = . + _heap_size;
    } >RAM AT>RAM :ram

    /* Stack space for each hart */
    . = ALIGN(4096);
    _stack_start = .;
    .stack : {
        . = . + (_hart_stack_size * (_max_hart_id + 1));
    } >RAM AT>RAM :ram
    _stack_end = .;

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }

    /* End of memory */
    . = ALIGN(4096);
    end = .;
}

/* Define symbols for memory management */
PROVIDE(_kernel_end = end);
PROVIDE(_heap_end = _heap_start + _heap_size);
PROVIDE(_stack_top = _stack_end); 