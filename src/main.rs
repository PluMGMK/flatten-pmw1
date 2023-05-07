extern crate pmw1;

use std::env::args;
use std::io::prelude::*;
use std::fs::File;

use pmw1::exe::Pmw1Exe;

const FIRST_USABLE_LOW_ADDX:u32 = 0xE000; // This is where PMODE/W loads stuff in fresh Dosbox
const LAST_USABLE_LOW_ADDX:u32 = 0xA0000; // Beginning of video memory
const FIRST_USABLE_HIGH_ADDX:u32 = 0x110000; // After HMA

const PAGESIZE:u32 = 0x1000;
const PAGEMASK:u32 = PAGESIZE - 1;

fn main() -> std::io::Result<()> {
    // Assume the filename of interest is the LAST argument on the command line.
    let exe_name: String = args().next_back().unwrap();

    // Load the whole EXE into memory...
    let binary = {
        println!("Opening {}...", exe_name);

        let mut file = File::open(&exe_name)?;
        let mut buffer: Vec<u8> = Vec::with_capacity(0x100000);
        file.read_to_end(&mut buffer)?;
        buffer.shrink_to_fit();
        buffer
    };

    println!("{} is {} bytes.", exe_name, binary.len());

    assert_eq!(binary[0..2],b"MZ"[..],
               "{} is not an MZ executable!", exe_name);
    assert!(binary.len() >= 0x1c,
            "{} doesn't appear to contain a complete MZ header!",exe_name);

    let mz_header = &binary[0x2..0x1c];
    let mz_header: Vec<u16> = (0..mz_header.len())
        .step_by(2)
        .map(|i| u16::from_le_bytes([mz_header[i], mz_header[i+1]]))
        .collect();

    // Print out some relevant info.
    println!("It begins with an MZ executable, of {} half-KiB blocks.",
             mz_header[1]);
    let total_block_size = mz_header[1] << 9; // Shift left to multiply by 512
    let actual_mz_size =
        if mz_header[0] == 0 {
            println!("Last block is fully used.");
            total_block_size
        } else {
            println!("{} bytes used in last block.", mz_header[0]);
            total_block_size - 512 + mz_header[0]
        } as usize;
    println!("Total MZ executable size is {} bytes.", actual_mz_size);

    assert!(binary.len() > actual_mz_size, "This appears to be a pure MZ executable!");

    // A slice containing just the PMW1 part.
    let pmw1_exe = Pmw1Exe::from_bytes(&binary[actual_mz_size..])?;

    // Is it all working??
    let pmw1_exe = pmw1_exe.decompress()?;

    // Allocate address space for the objects.
    let mut next_lomem_addx = FIRST_USABLE_LOW_ADDX;
    let mut next_himem_addx = FIRST_USABLE_HIGH_ADDX;
    let mut end_addx = FIRST_USABLE_LOW_ADDX;
    let object_bases: Vec<_> = pmw1_exe.iter_objects().map(|obj| {
        let virsize = obj.virtual_size() as u32;
        let virsize_roundup = (virsize + PAGEMASK) & (!PAGEMASK);
        let retval_ref = if virsize <= (LAST_USABLE_LOW_ADDX - next_lomem_addx) {
            // It'll fit in low memory
            &mut next_lomem_addx
        } else {
            // Gotta go in high memory!
            &mut next_himem_addx
        };
        let retval = *retval_ref;
        *retval_ref += virsize_roundup;
        // Update the end address for the map
        if retval + virsize > end_addx {
            end_addx = retval + virsize;
        }
        retval
    }).collect();

    // Dump the objects.
    let outfilename = format!("{}.FLAT",exe_name);
    let mut outfile = File::create(&outfilename)?;
    for (obj,objbase) in pmw1_exe.iter_objects().zip(&object_bases) {
        outfile.seek(std::io::SeekFrom::Start((*objbase).into()))?;
        let mut outdata = obj.data()?;
        for reloc in obj.iter_reloc_blocks().map(|b| b.iter_reloc_entries().unwrap()).flatten() {
            if reloc.rtype != 7 {
                println!("Ignoring unknown relocation type {}", reloc.rtype);
                continue;
            }
            let base = match object_bases.get(reloc.target_obj as usize - 1) {
                Some(&n) => n,
                None => 0,
            } as i32;
            let source_idx = reloc.source as usize;
            if source_idx+4 <= outdata.len() {
                outdata[source_idx..source_idx+4].copy_from_slice(&(reloc.target as i32 + base).to_le_bytes());
            } else {
                println!("Ignoring relocation at 0x{:X} hanging off the end of the file!", source_idx);
            }
        }
        outfile.write_all(&outdata)?;
    }
    outfile.set_len(end_addx.into())?;
    println!("Flat memory map written to {}", outfilename);

    let entry = pmw1_exe.entry_point();
    let stack = pmw1_exe.stack_pointer();
    println!("Entry point:   0x{:08X}", object_bases[(entry.0-1) as usize] + entry.1);
    println!("Stack pointer: 0x{:08X}", object_bases[(stack.0-1) as usize] + stack.1);

    Ok(())
}
