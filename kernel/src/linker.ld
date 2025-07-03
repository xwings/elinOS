/* Kernel memory layout for RISC-V 64-bit */
/* Bootloader loads kernel at 0x80400000 */
MEMORY {
    /* 2MB for kernel code and data */
    KERNEL (rwx) : ORIGIN = 0x80400000, LENGTH = 2M
    /* Flexible RAM region - provided by bootloader */
    RAM (rwx) : ORIGIN = 0x80600000, LENGTH = 58M
}

/* Define output sections */
PHDRS {
    text PT_LOAD FLAGS(5);  /* R-X */
    rodata PT_LOAD FLAGS(4);  /* R-- */
    data PT_LOAD FLAGS(6);  /* RW- */
    bss PT_LOAD FLAGS(6);  /* RW- */
    ram PT_LOAD FLAGS(6);  /* RW- */
}

/* Dynamic configuration - these are calculated at runtime based on available memory */
PROVIDE(_runtime_stack_size = 0);  /* Calculated dynamically per hart */
PROVIDE(_max_hart_id = 4);         /* Support up to 4 harts - can be detected */
PROVIDE(_runtime_hart_stack_size = 0);

/* Dynamic heap configuration - determined by memory manager */
PROVIDE(_runtime_heap_size = 0);   /* Calculated based on total available RAM */
PROVIDE(_min_heap_size = 64K);     /* Minimum heap size for safety */
PROVIDE(_min_stack_size = 16K);    /* Minimum stack size per hart */

OUTPUT_ARCH(riscv)
OUTPUT_FORMAT("elf64-littleriscv")
ENTRY(kernel_main)

SECTIONS {
    /* Kernel code and data at 0x80400000 */
    . = 0x80400000;

    .text : {
        __text_start = .;
        KEEP(*(.text.kernel))
        *(.text .text.*)
        __text_end = .;
    } :text

    .rodata : {
        __rodata_start = .;
        *(.rodata .rodata.*)
        __rodata_end = .;
    } :rodata

    .data : {
        __data_start = .;
        *(.data .data.*)
        __data_end = .;
    } :data

    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        __bss_end = .;
    } :bss

    /* Move to RAM region for heap and stack */
    . = 0x80600000;

    /* Dynamic memory regions - managed by unified memory manager */
    /* Note: Actual allocation is done at runtime, these are just placeholders */
    
    /* Reserve minimal space for linker symbols */
    _heap_start = .;
    .heap : {
        . = . + _min_heap_size;  /* Just reserve minimum space */
    } >RAM AT>RAM :ram
    
    /* Minimal stack reservation */
    . = ALIGN(4096);
    _stack_start = .;
    __stack_bottom = .;
    .stack : {
        . = . + (_min_stack_size * (_max_hart_id + 1));  /* Minimal reservation */
    } >RAM AT>RAM :ram
    _stack_end = .;
    __stack_top = .;

    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.debug*)
    }

    /* End of memory */
    . = ALIGN(4096);
    end = .;
}

/* Define symbols for memory management - static placeholders */
PROVIDE(_kernel_end = end);
PROVIDE(_static_heap_end = _heap_start + _min_heap_size);
PROVIDE(_static_stack_top = _stack_end);

/* Runtime memory detection symbols - filled by unified memory manager */
PROVIDE(_runtime_heap_start = 0);
PROVIDE(_runtime_heap_end = 0);
PROVIDE(_runtime_heap_size = 0);
PROVIDE(_runtime_stack_start = 0);
PROVIDE(_runtime_stack_end = 0);
PROVIDE(_runtime_stack_size = 0);
PROVIDE(_runtime_total_memory = 0);
PROVIDE(_runtime_available_memory = 0); 