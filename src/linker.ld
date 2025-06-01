/* Define memory regions */
MEMORY {
    RAM (rwx) : ORIGIN = 0x80200000, LENGTH = 128M
}

/* Define output sections */
PHDRS {
    text PT_LOAD FLAGS(5);  /* R-X */
    rodata PT_LOAD FLAGS(4);  /* R-- */
    data PT_LOAD FLAGS(6);  /* RW- */
    bss PT_LOAD FLAGS(6);  /* RW- */
}

/* Define the stack size */
PROVIDE(_stack_size = 65536);

/* Define the maximum number of harts */
PROVIDE(_max_hart_id = 0);

/* Define hart stack size */
PROVIDE(_hart_stack_size = _stack_size);

OUTPUT_ARCH(riscv)
OUTPUT_FORMAT("elf64-littleriscv")
ENTRY(_start)

SECTIONS {
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

    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }

    . = ALIGN(4096);
    end = .;
}

/* Define the start of the data section */
_sidata = LOADADDR(.data); 