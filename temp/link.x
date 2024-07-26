ENTRY(_start)
MEMORY
{
  rom : ORIGIN = 0xffffffff80000000, LENGTH = 0x12000
  ram : ORIGIN = 0xffffffff80012000, LENGTH = 0x12000
}

STACK_SIZE = 0x1000;

SECTIONS
{
  . = 0xffffffff80000000;

  .text   :
  {
    *(.start)
    . = 0xffffffff80000200;
    __IVT_START = ABSOLUTE(.);
    KEEP(*(.text.ivt))
    *(.text*)
  } > rom

  /* .rodata : { *(.rodata*) } > ram */

  .stack (NOLOAD) :
  {
    . = 0xffffffff80016000;
    . = ALIGN(8);
    . = . + STACK_SIZE;
    . = ALIGN(8);
  } > ram
}
