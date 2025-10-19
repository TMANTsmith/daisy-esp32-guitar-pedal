/* memory layout for ESP32-S3 */
MEMORY
{
    IRAM0   (rx) : ORIGIN = 0x40000000, LENGTH = 512K
    DRAM0   (rwx) : ORIGIN = 0x3FF80000, LENGTH = 512K
    FLASH   (rx) : ORIGIN = 0x08000000, LENGTH = 4M
}

/* sections */
SECTIONS
{
    /* text/code section */
    .text : ALIGN(16) {
        KEEP(*(.vector_table .vector_table.*)) /* startup vectors */
        *(.text .text.*)
        *(.rodata .rodata.*)
    } > IRAM0

    /* initialized data */
    .data : ALIGN(4) {
        _sdata = .;
        *(.data .data.*)
        _edata = .;
    } > DRAM0 AT > FLASH

    /* zeroed data */
    .bss : ALIGN(4) {
        _sbss = .;
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
    } > DRAM0

    /* stack (top of DRAM) */
    .stack (NOLOAD) : ALIGN(16) {
        _stack_start = .;
        . = . + 0x4000; /* 16 KB stack */
        _stack_end = .;
    } > DRAM0
}

