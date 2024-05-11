ENTRY(_start)
MEMORY
{
  rom : ORIGIN = 0xffffffff80000000, LENGTH = 0x2000
  ram : ORIGIN = 0xffffffff80002000, LENGTH = 0x3000
}

STACK_SIZE = 0x1000;

SECTIONS
{
  . = 0xffffffff80000000;

  .text   :
  {
    *(.start)
    *(.text*)
  } > rom

  /* .rodata : { *(.rodata*) } > ram */

  .stack (NOLOAD) :
  {
    . = 0xffffffff80003000;
    . = ALIGN(8);
    . = . + STACK_SIZE;
    . = ALIGN(8);
  } > ram
}
