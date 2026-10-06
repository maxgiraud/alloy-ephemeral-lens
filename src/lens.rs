
use alloy::{
    dyn_abi::{FunctionExt, SolType},
    eips::BlockId,
    network::Network, primitives::{Address, Bytes, B256, U256},
    providers::Provider, rpc::types::state::{AccountOverride, StateOverride},
    sol_types::{JsonAbiExt, SolCall}
};

use crate::{call::Call, contract::IProxy::{self, IProxyInstance}, CallResult};

/// A struct that acts as a lens to interact with a smart contract proxy
pub struct Lens<P, N>
where
    N: Network,
    P: Provider<N>
{
    /// Proxy contract instance
    proxy: IProxyInstance<P, N>,
    /// Collection of contract calls
    calls: Vec<Call>,
    /// State overrides for ephemeral execution
    state_overrides: StateOverride,
    /// `from` address for the eth_call (sets `tx.origin`); `None` uses the
    /// provider default.
    caller: Option<Address>,
    /// Block the eth_call executes against; `None` uses the provider default
    /// (`latest`).
    block: Option<BlockId>,
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
    /// let provider = ProviderBuilder::new().connect("http://localhost:8080").await.unwrap();
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
            caller: None,
            block: None,
        }
    }

    /// Sets the `from` address of the underlying eth_call, which the EVM exposes
    /// as `tx.origin`. Required for contracts that gate on `tx.origin` (e.g. the
    /// arbitrage executor's owner check).
    pub fn with_caller(&mut self, from: Address) -> &mut Self {
        self.caller = Some(from);
        self
    }

    /// Pins the underlying eth_call to `block`, so every call reads that block's
    /// state (requires an archive node for old blocks). State overrides still
    /// apply on top of it. The ephemeral proxy uses post-Cancun opcodes, so a
    /// block before Cancun (mainnet 19,426,587) fails with `NotActivated`.
    pub fn with_block(&mut self, block: BlockId) -> &mut Self {
        self.block = Some(block);
        self
    }

    /// Relocates the lens proxy to `address`, moving its code state-override with
    /// it. The proxy reaches each target via `CALL`, so its address becomes the
    /// target's `msg.sender` — set this to a contract's expected owner/caller
    /// when that contract gates on `msg.sender` (e.g. the arbitrage executor's
    /// `IS_OWNER` check, distinct from its `tx.origin` gate handled by
    /// [`Self::with_caller`]).
    pub fn with_proxy_address(&mut self, address: Address) -> &mut Self
    where
        P: Clone,
    {
        let old = *self.proxy.address();
        relocate_proxy(&mut self.state_overrides, old, address);
        let provider = self.proxy.provider().clone();
        self.proxy = IProxyInstance::new(address, provider);
        self
    }

    /// Registers a call carrying pre-encoded `calldata` to `address`, bypassing
    /// `SolCall` ABI-encoding. Use for contracts with non-standard calldata
    /// (custom selectors, tightly-packed args). The raw return bytes are exposed
    /// on [`CallResult::raw`] for the caller to decode.
    pub fn with_raw_call(&mut self, address: &Address, calldata: Bytes) -> &mut Self {
        self.calls.push(Call::new(|_| vec![], *address, calldata));
        self
    }

    /// Like [`Self::with_raw_call`], but caps the gas forwarded to this call.
    /// The proxy executes it as `callee.call{gas: gas}(calldata)`, so a heavy or
    /// griefing call cannot starve the other items in the same batch. A `gas` of
    /// zero forwards all remaining gas (identical to [`Self::with_raw_call`]).
    pub fn with_raw_call_gas(&mut self, address: &Address, calldata: Bytes, gas: U256) -> &mut Self {
        let mut call = Call::new(|_| vec![], *address, calldata);
        call.with_gas(gas);
        self.calls.push(call);
        self
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

    /// Overrides the ether balance of `address` for the duration of the call.
    ///
    /// Merges into any existing override for `address` (e.g. a code override set
    /// by [`Self::with_ephemeral`]), leaving its other fields intact.
    pub fn with_balance(&mut self, address: &Address, balance: U256) -> &mut Self {
        insert_balance(&mut self.state_overrides, *address, balance);
        self
    }

    /// Overrides a single storage `slot` of `address` to `value` for the call,
    /// accumulating into that account's state diff. Used to fund ERC20 balances
    /// by patching the `balanceOf` mapping slot. Merges with any existing
    /// override (code/balance/other slots) for `address`.
    pub fn with_storage(&mut self, address: &Address, slot: B256, value: B256) -> &mut Self {
        insert_storage(&mut self.state_overrides, *address, slot, value);
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
    /// # let provider = ProviderBuilder::new().connect("http://localhost:8080").await.unwrap();
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
            Call::new(
                // A non-reverting call can still return data that doesn't match the
                // expected output ABI (e.g. calling `balanceOf` on a non-ERC20
                // address). Decode leniently — yield no typed values rather than
                // panicking the whole batch; callers inspect `result`/`raw`.
                |data| T::abi().abi_decode_output(data).unwrap_or_default(),
                *address,
                call.abi_encode().into()
            )
        );

        self
    }

    /// Registers multiple calls of the same type to the same address with different args
    pub fn with_multicall<T>(&mut self, address: &Address, args_list: Vec<<T::Parameters<'_> as SolType>::RustType>) -> &mut Self
    where
        T: SolCall + JsonAbiExt,
        T::Abi: FunctionExt,
    {
        for args in args_list {
            self.with_call::<T>(address, args);
        }
        self
    }

    /// Executes all registered calls and collects their results.
    ///
    /// Returns an error if the underlying `eth_call` fails (e.g. transport
    /// error, or the node's gas cap is exceeded). A per-call revert is *not* an
    /// error here — it is captured in the corresponding [`CallResult`].
    pub async fn call(&self) -> Result<Vec<CallResult>, alloy::contract::Error> {
        let calls = self.calls.iter()
            .map(|elt| elt.encode())
            .collect();

        let mut builder = self.proxy.execute(calls).state(self.state_overrides.clone());
        if let Some(from) = self.caller {
            builder = builder.from(from);
        }
        if let Some(block) = self.block {
            builder = builder.block(block);
        }
        let result = builder.call().await?;

        Ok(self.calls.iter()
            .zip(result.iter())
            .map(|(c, elt)| CallResult::from(c, elt))
            .collect())
    }
}

/// Set the balance override for `address`, preserving any other fields already
/// set on its [`AccountOverride`].
fn insert_balance(overrides: &mut StateOverride, address: Address, balance: U256) {
    overrides.entry(address).or_default().balance = Some(balance);
}

/// Add a single storage slot to `address`'s state diff, preserving any other
/// fields and slots already set on its [`AccountOverride`].
fn insert_storage(overrides: &mut StateOverride, address: Address, slot: B256, value: B256) {
    overrides
        .entry(address)
        .or_default()
        .state_diff
        .get_or_insert_with(Default::default)
        .insert(slot, value);
}

/// Move the proxy code override from `old` to `new`: drop the stale entry and
/// inject the proxy bytecode at the new address. Its other overrides are not
/// touched (the proxy carries only code).
fn relocate_proxy(overrides: &mut StateOverride, old: Address, new: Address) {
    overrides.remove(&old);
    overrides.insert(
        new,
        AccountOverride::default().with_code(IProxy::DEPLOYED_BYTECODE.clone()),
    );
}

#[cfg(test)]
mod override_tests {
    use super::*;

    #[test]
    fn balance_and_storage_merge_into_one_account() {
        let mut overrides = StateOverride::default();
        let token = Address::repeat_byte(0xaa);

        insert_balance(&mut overrides, token, U256::from(42u64));
        insert_storage(&mut overrides, token, B256::ZERO, B256::with_last_byte(7));
        insert_storage(&mut overrides, token, B256::with_last_byte(1), B256::with_last_byte(9));

        let acct = overrides.get(&token).expect("account override present");
        assert_eq!(acct.balance, Some(U256::from(42u64)));
        let diff = acct.state_diff.as_ref().expect("state diff present");
        assert_eq!(diff.get(&B256::ZERO), Some(&B256::with_last_byte(7)));
        assert_eq!(diff.get(&B256::with_last_byte(1)), Some(&B256::with_last_byte(9)));
    }

    #[test]
    fn balance_override_keeps_existing_code() {
        let mut overrides = StateOverride::default();
        let addr = Address::repeat_byte(0xbb);
        overrides.insert(addr, AccountOverride::default().with_code(Bytes::from_static(&[0x60])));

        insert_balance(&mut overrides, addr, U256::from(1u64));

        let acct = overrides.get(&addr).unwrap();
        assert_eq!(acct.balance, Some(U256::from(1u64)));
        assert!(acct.code.is_some(), "code override must survive a later balance override");
    }

    #[test]
    fn relocate_proxy_moves_code_override_to_new_address() {
        let old = Address::repeat_byte(0x01);
        let new = Address::repeat_byte(0x99);
        let mut overrides = StateOverride::default();
        overrides.insert(old, AccountOverride::default().with_code(IProxy::DEPLOYED_BYTECODE.clone()));

        relocate_proxy(&mut overrides, old, new);

        assert!(overrides.get(&old).is_none(), "stale proxy override must be removed");
        let acct = overrides.get(&new).expect("proxy override present at new address");
        assert_eq!(acct.code.as_ref(), Some(&IProxy::DEPLOYED_BYTECODE.clone()));
    }
}