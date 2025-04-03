use alloy::{
    dyn_abi::{DynSolType, DynSolValue, FunctionExt, SolType},
    network::Network, primitives::{Address, Bytes, U256},
    providers::Provider, rpc::types::state::{AccountOverride, StateOverride},
    sol_types::{JsonAbiExt, Revert, SolCall, SolError}
};

use crate::contract::IProxy::{self, IProxyInstance};


/// Represents a contract call with encoding and decoding functionalities
struct Call {
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

    fn encode(&self) -> IProxy::CallArgument {
        IProxy::CallArgument {
            callee: self.address,
            argument: self.argument.clone(),
            value: self.value,
            gas: self.gas,
        }
    }
        
    fn decode(&self, data: &[u8]) -> Vec<DynSolValue> {
        (self.decoder)(data)
    }
}

/// A struct that acts as a lens to interact with a smart contract proxy
pub struct Lens<P, N>
where
    N: Network,
    P: Provider<N>
{
    /// Proxy contract instance
    proxy: IProxyInstance<(), P, N>,
    /// Collection of contract calls
    calls: Vec<Call>,
    /// State overrides for ephemeral execution
    state_overrides: StateOverride,
}

impl<P, N> Lens<P, N>
where
    N: Network,
    P: Provider<N>
{

    /// Constructs a new `Lens` instance with an initialized proxy contract.
    ///
    /// # Example
    /// ```
    /// # use alloy_ephemeral_lens::Lens;
    /// # use alloy::providers::ProviderBuilder;
    /// # tokio_test::block_on(async {
    /// let provider = ProviderBuilder::new().on_builtin("http://localhost:8080").await.unwrap();
    /// let lens = Lens::new(&provider);
    /// # })
    /// ```
    pub fn new(provider: P) -> Self {
        let proxy_address = Address::repeat_byte(0x01);
        let mut state_override = StateOverride::default();
        state_override.insert(
            proxy_address,
            AccountOverride::default().with_code(IProxy::DEPLOYED_BYTECODE.clone())
        );

        Self {
            proxy: IProxyInstance::new(proxy_address, provider),
            calls: vec![],
            state_overrides: state_override,
        }
    }

    /// Adds an ephemeral contract to the state override for execution
    /// 
    /// This could be for an ephemeral lens contract or an interacted contract
    /// 
    pub fn with_ephemeral(&mut self, address: &Address, run_bytecode: Bytes) -> &mut Self {
        self.state_overrides.insert(
            *address,
            AccountOverride::default().with_code(run_bytecode)
        );

        self
    }

    /// Registers a contract call via the `Proxy` to the contract at `address` with `args`
    /// 
    /// # Example
    /// ```
    /// # use alloy_ephemeral_lens::Lens;
    /// # use alloy::{hex::FromHex, primitives::{address,Address}, providers::{ProviderBuilder, WsConnect}, sol};
    /// #
    /// sol! {
    ///     interface LensContract {
    ///         #[sol(abi)] // needed
    ///         function getTokenSymbol(address) external view returns (string);
    ///     }
    /// }
    /// #
    /// # tokio_test::block_on(async {
    /// # let provider = ProviderBuilder::new().on_builtin("http://localhost:8080").await.unwrap();
    /// # let address = Address::repeat_byte(0x01);
    /// # let mut lens = Lens::new(&provider);
    /// 
    /// let weth = address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    /// 
    /// // Add a call to `getTokenSymbol` to a real or ephemeral contract at `address`
    /// // with the `weth` address as argument
    /// lens.with_call::<LensContract::getTokenSymbolCall>(&address,(weth,))
    /// # ;
    /// # })
    /// ```
    pub fn with_call<T>(&mut self, address: &Address, args: <T::Parameters<'_> as SolType>::RustType) -> &mut Self
    where 
        T: SolCall + JsonAbiExt,
        T::Abi: FunctionExt
    {
        let call = T::new(args);
        self.calls.push(
            Call {
                decoder: |data| T::abi().abi_decode_output(data, true).unwrap(),
                address: *address,
                argument: call.abi_encode().into(),
                value: U256::from(0),
                gas: U256::from(1000000000u32),
            }
        );

        self
    }

    /// Executes all registered calls and collects their results
    pub async fn call(&self) -> Vec<CallResult> {
        let calls = self.calls.iter()
            .map(|elt| elt.encode())
            .collect();

        let result = self.proxy.execute(calls).state(self.state_overrides.clone()).call().await.unwrap();
        
        self.calls.iter()
            .zip(result._0.iter())
            .map(|(c, elt)| CallResult::from(c, elt))
            .collect()
    }
}

/// Represents the result of a contract call
#[derive(Debug)]
pub struct CallResult {
    /// Indicates if the call was successful
    pub success: bool,
    /// Gas consumed by the call
    pub gas_used: U256,
    /// Decoded return data
    pub result: Vec<DynSolValue>,
    /// Error details if the call reverted
    pub revert: Option<Revert>,
}

impl CallResult {
    /// Constructs a CallResult instance from raw response data
    fn from(call: &Call, data: &Bytes) -> Self {
        let binding = DynSolType::Bytes.abi_decode(&data[4..]).unwrap();
        let result_data = binding.as_bytes().unwrap();

        let binding = DynSolType::Tuple(
            vec![DynSolType::Bool, DynSolType::Uint(256), DynSolType::Bytes]
        ).abi_decode_params(result_data)
        .unwrap();

        let result_data = binding.as_tuple().unwrap();

        let success = result_data[0].as_bool().unwrap();
        let gas_used = result_data[1].as_uint().unwrap().0.into();

        let result = if success {
            call.decode(result_data[2].as_bytes().unwrap())
        } else {
            vec![]
        };

        let revert = if success {
            None
        } else {
            Some(Revert::abi_decode(result_data[2].as_bytes().unwrap(), true).unwrap())
        };

        Self { success, gas_used, result, revert }
    }
}