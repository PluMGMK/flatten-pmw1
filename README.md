flatten-pmw1
============

This is a simple program to help with reverse-engineering of DOS EXEs in the PMW1 format. PMW1 is the compressible EXE format used by the [PMODE/W DOS Extender](http://www.sid6581.net/pmodew/).

You can simply pass it a PMW1 EXE on the command line, and it creates a flat map of the EXE's contents, which is then saved to a file with the original EXE's name plus the suffix `.FLAT`. This can then be disassembled as a simple binary file. To create the flat map, each object in the EXE is decompressed and placed at an offset which is a multiple of `0x1000` (386 page size). The relocations are all applied, so if you load the flat map into a disassembler, all the addresses will be correct!

To avoid confusion over any code in the EXE which accesses BIOS or Video memory, objects are not placed in the range `0 - 0xE000` or `0xA0000 - 0x101000`. Empty space in the map can be ignored when you disassemble the file. It will be filled with zeros, and may or may not take up disk space, depending on your OS and file-system.

# Usage

If you want to compile it yourself:
```
$ git clone https://github.com/PluMGMK/flatten-pmw1.git
$ cd flatten-pmw1
$ cargo run --release -- /PATH/TO/PMW.EXE
```
This will then create `/PATH/TO/PMW.EXE.FLAT`, which you can disassemble as a binary file.

Otherwise, you can download the pre-compiled EXE (Win64 since Rust/LLVM have made it harder to compile `pmw1-rs` for 32-bit since 2020…) and run it. On Windows, you can just drag a PMW1 EXE onto it in the GUI, but I recommend running it from the command line because then you can read the entry point and stack pointer in the output.
