use fuel_asm::RegId;
use fuel_vm::{
    error::SimpleResult,
    prelude::{Interpreter, MemoryRange},
};

pub fn type_id<S, Tx>(
    vm: &mut Interpreter<S, Tx, super::MyEcal>,
    rb: RegId,
) -> SimpleResult<()> {
    let type_name: String = {
        // r_b: the address of (address, lenght)
        let addr = vm.registers()[rb];
        // read the tuple stored as two consecutive u64 values
        let r = MemoryRange::new(addr, 2 * std::mem::size_of::<u64>())?;
        let bytes: [u8; 2 * std::mem::size_of::<u64>()] =
            vm.memory()[r.usizes()].try_into().unwrap();
        // convert to (address, length) of the string to be printed
        let (addr, len): (u64, u64) =
            fuels::core::codec::try_from_bytes(&bytes, super::DECODER_CONFIG).unwrap();
        // read the string
        let r = MemoryRange::new(addr, len)?;
        let bytes = vm.memory()[r.usizes()].to_vec();
        String::from_utf8(bytes).unwrap()
    };

    let type_id = vm.ecal_state_mut().abi.type_id(&type_name);

    vm.registers_mut()[rb] = type_id as u64;

    Ok(())
}
