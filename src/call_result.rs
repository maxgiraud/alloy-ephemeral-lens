

use alloy::{dyn_abi::{DynSolType, DynSolValue}, primitives::{Bytes, U256}, sol_types::{Revert, SolError}};

use crate::call::Call;


/// Represents the result of a contract call
#[derive(Debug)]
pub struct CallResult {
    /// Indicates if the call was successful
    pub success: bool,
    /// Gas consumed by the call
    pub gas_used: U256,
    /// Decoded return data
    pub result: Vec<DynSolValue>,
    /// Raw return bytes, before any typed decoding. Lets callers decode results
    /// the typed `decoder` does not cover (e.g. raw calls).
    pub raw: Bytes,
    /// Error details if the call reverted
    pub revert: Option<Revert>,
}

impl CallResult {
    /// Constructs a CallResult instance from raw response data
    pub(super) fn from(call: &Call, data: &Bytes) -> Self {
        let binding = DynSolType::Bytes.abi_decode(&data[4..]).unwrap();
        let result_data = binding.as_bytes().unwrap();

        let binding = DynSolType::Tuple(
            vec![DynSolType::Bool, DynSolType::Uint(256), DynSolType::Bytes]
        ).abi_decode_params(result_data)
        .unwrap();

        let result_data = binding.as_tuple().unwrap();

        let success = result_data[0].as_bool().unwrap();
        let gas_used = result_data[1].as_uint().unwrap().0;
        let raw_bytes = result_data[2].as_bytes().unwrap();

        let result = if success {
            call.decode(raw_bytes)
        } else {
            vec![]
        };

        let revert = if success {
            None
        } else {
            Revert::abi_decode(raw_bytes).ok()
        };

        Self { success, gas_used, result, raw: Bytes::copy_from_slice(raw_bytes), revert }
    }
}