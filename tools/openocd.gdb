target extended-remote :3333

# TODO(blakely): Is this necessary when using cortex-debug?
set print asm-demangle on

# Disabled for now, since we've only got six breakpoints total. Detect hard
# faults, unhandled exceptions, and panics break DefaultHandler break
# UserHardFault break rust_begin_unwind

# Send captured ITM to the file itm.fifo Final number must match the core clock
# frequency at the time you want to record output, which means it should be 
# changed to 16MHz if you want to record messages pre-boot, or 170MHz after
# clock config. 

monitor tpiu config internal itm.txt uart off 16000000
# monitor tpiu config internal itm.txt uart off 170000000

# enable ITM port 0
monitor itm port 0 on

load