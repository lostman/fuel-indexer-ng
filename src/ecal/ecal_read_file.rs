use fuel_asm::RegId;
use fuel_vm::{
    error::SimpleResult,
    prelude::{Interpreter, MemoryRange},
};

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

fn read_file_ecal<S, Tx>(
    vm: &mut Interpreter<S, Tx, Self>,
    _ra: RegId,
    rb: RegId,
    _rc: RegId,
    _rd: RegId,
) -> SimpleResult<()> {
    let args: (u64, u64, u64, u64) = {
        let addr = vm.registers()[rb];
        let r = MemoryRange::new(addr, 4 * 8)?;
        let bytes: [u8; 4 * 8] = vm.memory()[r.usizes()].try_into().unwrap();
        fuels::core::codec::try_from_bytes(&bytes, DECODER_CONFIG).unwrap()
    };

    #[cfg(debug_assertions)]
    println!("read_file args = {args:?}");

    vm.gas_charge(args.1.saturating_add(1))?;

    // Extract file path from vm memory
    let path = {
        let r = MemoryRange::new(args.2, args.3)?;
        let path = String::from_utf8_lossy(&vm.memory()[r.usizes()]);
        let path = PathBuf::from(path.as_ref());

        #[cfg(debug_assertions)]
        println!("read_file file_path = {path:?}");

        path
    };

    // Seek file to correct position
    let mut file = File::open(path).map_err(|_| PanicReason::EcalError)?;
    let _ = file
        .seek(SeekFrom::Start(args.0))
        .map_err(|_| PanicReason::EcalError)?;

    // Allocate the buffer in the vm memory and read directly from the file into it
    let output = {
        vm.allocate(args.1)?;
        let r: MemoryRange = MemoryRange::new(vm.registers()[RegId::HP], args.1)?;
        let len = file
            .read(&mut vm.memory_mut()[r.usizes()])
            .map_err(|_| PanicReason::EcalError)?;

        #[cfg(debug_assertions)]
        println!("read_file read {len} bytes");

        (r.start as u64, len as u64)
    };

    let output_bytes: Vec<u8> =
        fuels::core::codec::calldata!(output).expect("Failed to encode output tuple");
    vm.allocate(output_bytes.len() as u64)?;
    let o = MemoryRange::new(vm.registers()[RegId::HP], output_bytes.len())?;

    #[cfg(debug_assertions)]
    println!("output = {} {:?}", o.start, o.usizes());
    vm.memory_mut()[o.usizes()].copy_from_slice(&output_bytes);

    // Return the address of the output tuple through the rB register
    vm.registers_mut()[rb] = o.start as u64;

    Ok(())
}
