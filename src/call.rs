use alloy::{
    dyn_abi::DynSolValue,
    primitives::{Address, Bytes, U256}
};

use crate::contract::IProxy::{self};


/// Represents a contract call with encoding and decoding functionalities
pub struct Call {
    /// Function pointer for decoding response data
    decoder: fn(&[u8]) -> Vec<DynSolValue>,
    /// Address of the contract being called
    address: Address,
    /// Encoded function arguments
    argument: Bytes,
    /// Ether value sent with the call
    value: U256,
    /// Gas limit for the call
    gas: U256,
}

impl Call {

    pub fn new(decoder: fn(&[u8]) -> Vec<DynSolValue>, address: Address, argument: Bytes) -> Self {
        Self { decoder, address, argument, value: U256::ZERO, gas: U256::ZERO }
    }

    /// TODO: unused
    pub fn with_value(&mut self, value: U256) -> &mut Self {
        self.value = value;
        self
    }

    /// TODO: unused
    pub fn with_gas(&mut self, gas: U256) -> &mut Self {
        self.gas = gas;
        self
    }

    pub(super) fn encode(&self) -> IProxy::CallArgument {
        IProxy::CallArgument {
            callee: self.address,
            argument: self.argument.clone(),
            value: self.value,
            gas: self.gas,
        }
    }
        
    pub(super) fn decode(&self, data: &[u8]) -> Vec<DynSolValue> {
        (self.decoder)(data)
    }
}