use std::env;

use alloy::{
    eips::BlockId,
    primitives::{address, Address, Bytes, U256},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{BlockOverrides, TransactionRequest},
    sol,
    sol_types::SolCall,
};
use alloy_ephemeral_lens::Lens;

// Direct ERC20 interface — no bytecode, calls go straight to mainnet contracts
sol! {
    interface IERC20 {
        #[sol(abi)]
        function name() external view returns (string memory);
        #[sol(abi)]
        function symbol() external view returns (string memory);
        #[sol(abi)]
        function decimals() external view returns (uint8);
        #[sol(abi)]
        function totalSupply() external view returns (uint256);
    }
}

// Ephemeral lens that aggregates name/symbol/decimals in a single call
// Source: examples/ERC20_metadata/TokenLens.sol
sol! {
    #[sol(deployed_bytecode="608060405234801561000f575f5ffd5b5060043610610029575f3560e01c8063597704381461002d575b5f5ffd5b6100476004803603810190610042919061022d565b610060565b60405161005794939291906102f2565b60405180910390f35b5f6060805f848573ffffffffffffffffffffffffffffffffffffffff166306fdde036040518163ffffffff1660e01b81526004015f60405180830381865afa1580156100ae573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f820116820180604052508101906100d69190610461565b8673ffffffffffffffffffffffffffffffffffffffff166395d89b416040518163ffffffff1660e01b81526004015f60405180830381865afa15801561011e573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f820116820180604052508101906101469190610461565b8773ffffffffffffffffffffffffffffffffffffffff1663313ce5676040518163ffffffff1660e01b8152600401602060405180830381865afa15801561018f573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906101b391906104d2565b93509350935093509193509193565b5f604051905090565b5f5ffd5b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6101fc826101d3565b9050919050565b61020c816101f2565b8114610216575f5ffd5b50565b5f8135905061022781610203565b92915050565b5f60208284031215610242576102416101cb565b5b5f61024f84828501610219565b91505092915050565b610261816101f2565b82525050565b5f81519050919050565b5f82825260208201905092915050565b8281835e5f83830152505050565b5f601f19601f8301169050919050565b5f6102a982610267565b6102b38185610271565b93506102c3818560208601610281565b6102cc8161028f565b840191505092915050565b5f60ff82169050919050565b6102ec816102d7565b82525050565b5f6080820190506103055f830187610258565b8181036020830152610317818661029f565b9050818103604083015261032b818561029f565b905061033a60608301846102e3565b95945050505050565b5f5ffd5b5f5ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b6103818261028f565b810181811067ffffffffffffffff821117156103a05761039f61034b565b5b80604052505050565b5f6103b26101c2565b90506103be8282610378565b919050565b5f67ffffffffffffffff8211156103dd576103dc61034b565b5b6103e68261028f565b9050602081019050919050565b5f610405610400846103c3565b6103a9565b90508281526020810184848401111561042157610420610347565b5b61042c848285610281565b509392505050565b5f82601f83011261044857610447610343565b5b81516104588482602086016103f3565b91505092915050565b5f60208284031215610476576104756101cb565b5b5f82015167ffffffffffffffff811115610493576104926101cf565b5b61049f84828501610434565b91505092915050565b6104b1816102d7565b81146104bb575f5ffd5b50565b5f815190506104cc816104a8565b92915050565b5f602082840312156104e7576104e66101cb565b5b5f6104f4848285016104be565b9150509291505056fea2646970667358221220d5d294ac677abffccafbb280cacd721e1a7c7a47be2c649ef5fa01609c9f721364736f6c634300081b0033")]
    interface ITokenLens {
        #[sol(abi)]
        function getToken(address token) external view returns (address, string, string, uint8);
    }
}

// Always-reverting lens for testing revert isolation
sol! {
    #[sol(deployed_bytecode="608060405234801561000f575f5ffd5b506004361061003f575f3560e01c806304d91c6a146100435780635977043814610061578063f8a8fd6d14610094575b5f5ffd5b61004b6100b2565b60405161005891906102fe565b60405180910390f35b61007b60048036038101906100769190610389565b6100ef565b60405161008b94939291906103de565b60405180910390f35b61009c610251565b6040516100a991906102fe565b60405180910390f35b60606040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016100e690610479565b60405180910390fd5b5f6060805f848573ffffffffffffffffffffffffffffffffffffffff166306fdde036040518163ffffffff1660e01b81526004015f60405180830381865afa15801561013d573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f8201168201806040525081019061016591906105b5565b8673ffffffffffffffffffffffffffffffffffffffff166395d89b416040518163ffffffff1660e01b81526004015f60405180830381865afa1580156101ad573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f820116820180604052508101906101d591906105b5565b8773ffffffffffffffffffffffffffffffffffffffff1663313ce5676040518163ffffffff1660e01b8152600401602060405180830381865afa15801561021e573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906102429190610626565b93509350935093509193509193565b60606040518060400160405280600681526020017f636f75636f750000000000000000000000000000000000000000000000000000815250905090565b5f81519050919050565b5f82825260208201905092915050565b8281835e5f83830152505050565b5f601f19601f8301169050919050565b5f6102d08261028e565b6102da8185610298565b93506102ea8185602086016102a8565b6102f3816102b6565b840191505092915050565b5f6020820190508181035f83015261031681846102c6565b905092915050565b5f604051905090565b5f5ffd5b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6103588261032f565b9050919050565b6103688161034e565b8114610372575f5ffd5b50565b5f813590506103838161035f565b92915050565b5f6020828403121561039e5761039d610327565b5b5f6103ab84828501610375565b91505092915050565b6103bd8161034e565b82525050565b5f60ff82169050919050565b6103d8816103c3565b82525050565b5f6080820190506103f15f8301876103b4565b818103602083015261040381866102c6565b9050818103604083015261041781856102c6565b905061042660608301846103cf565b95945050505050565b7f41696520636f75702064757220706f7572206775696c6c61756d6500000000005f82015250565b5f610463601b83610298565b915061046e8261042f565b602082019050919050565b5f6020820190508181035f83015261049081610457565b9050919050565b5f5ffd5b5f5ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b6104d5826102b6565b810181811067ffffffffffffffff821117156104f4576104f361049f565b5b80604052505050565b5f61050661031e565b905061051282826104cc565b919050565b5f67ffffffffffffffff8211156105315761053061049f565b5b61053a826102b6565b9050602081019050919050565b5f61055961055484610517565b6104fd565b9050828152602081018484840111156105755761057461049b565b5b6105808482856102a8565b509392505050565b5f82601f83011261059c5761059b610497565b5b81516105ac848260208601610547565b91505092915050565b5f602082840312156105ca576105c9610327565b5b5f82015167ffffffffffffffff8111156105e7576105e661032b565b5b6105f384828501610588565b91505092915050565b610605816103c3565b811461060f575f5ffd5b50565b5f81519050610620816105fc565b92915050565b5f6020828403121561063b5761063a610327565b5b5f61064884828501610612565b9150509291505056fea2646970667358221220ed60340e334891abf19b2a56f5de2ec999aa1bf63e723ba46959bb975cd0050a64736f6c634300081b0033")]
    interface IRevertLens {
        #[sol(abi)]
        function testFail() external view returns (string memory);
    }
}

// Multicall3 exposes the block environment as views.
sol! {
    interface IMulticall3 {
        #[sol(abi)]
        function getBlockNumber() external view returns (uint256);
        #[sol(abi)]
        function getCurrentBlockTimestamp() external view returns (uint256);
        #[sol(abi)]
        function getBasefee() external view returns (uint256);
    }
}

const MULTICALL3: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");
const WETH: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const DAI:  Address = address!("6B175474E89094C44Da98b954EedeAC495271d0F");

macro_rules! require_provider {
    () => {{
        let rpc_url = match env::var("RPC_URL") {
            Ok(url) => url,
            Err(_) => { eprintln!("Skipping: RPC_URL not set"); return; }
        };
        ProviderBuilder::new().connect_ws(WsConnect::new(rpc_url)).await.unwrap()
    }};
}

/// Batch multiple direct calls to well-known ERC20s in a single eth_call,
/// asserting exact on-chain values.
#[tokio::test]
async fn test_batch_direct_erc20_calls() {
    let provider = require_provider!();

    let mut lens = Lens::new(&provider);
    lens.with_call::<IERC20::nameCall>(&WETH, ())
        .with_call::<IERC20::symbolCall>(&WETH, ())
        .with_call::<IERC20::decimalsCall>(&WETH, ())
        .with_call::<IERC20::nameCall>(&USDC, ())
        .with_call::<IERC20::symbolCall>(&USDC, ())
        .with_call::<IERC20::decimalsCall>(&USDC, ());

    let results = lens.call().await.unwrap();

    assert_eq!(results.len(), 6);
    assert!(results.iter().all(|r| r.success), "all calls should succeed");

    assert_eq!(results[0].result[0].as_str().unwrap(), "Wrapped Ether");
    assert_eq!(results[1].result[0].as_str().unwrap(), "WETH");
    assert_eq!(results[2].result[0].as_uint().unwrap().0, U256::from(18u8));

    assert_eq!(results[3].result[0].as_str().unwrap(), "USD Coin");
    assert_eq!(results[4].result[0].as_str().unwrap(), "USDC");
    assert_eq!(results[5].result[0].as_uint().unwrap().0, U256::from(6u8));
}

/// Deploy an ephemeral aggregator lens and use it to fetch metadata for three
/// tokens in a single batch — no separate calls for name/symbol/decimals.
#[tokio::test]
async fn test_ephemeral_aggregator_lens() {
    let provider = require_provider!();
    let lens_addr = Address::repeat_byte(0xca);

    let mut lens = Lens::new(&provider);
    lens.with_ephemeral(&lens_addr, ITokenLens::DEPLOYED_BYTECODE.clone())
        .with_call::<ITokenLens::getTokenCall>(&lens_addr, (WETH,))
        .with_call::<ITokenLens::getTokenCall>(&lens_addr, (USDC,))
        .with_call::<ITokenLens::getTokenCall>(&lens_addr, (DAI,));

    let results = lens.call().await.unwrap();

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));

    // [0] = token address, [1] = name, [2] = symbol, [3] = decimals
    assert_eq!(results[0].result[1].as_str().unwrap(), "Wrapped Ether");
    assert_eq!(results[0].result[2].as_str().unwrap(), "WETH");
    assert_eq!(results[0].result[3].as_uint().unwrap().0, U256::from(18u8));

    assert_eq!(results[1].result[1].as_str().unwrap(), "USD Coin");
    assert_eq!(results[1].result[2].as_str().unwrap(), "USDC");
    assert_eq!(results[1].result[3].as_uint().unwrap().0, U256::from(6u8));

    assert_eq!(results[2].result[1].as_str().unwrap(), "Dai Stablecoin");
    assert_eq!(results[2].result[2].as_str().unwrap(), "DAI");
    assert_eq!(results[2].result[3].as_uint().unwrap().0, U256::from(18u8));
}

/// A reverting call in the middle of a batch must not abort subsequent calls.
/// This is the core guarantee of the proxy executor.
#[tokio::test]
async fn test_revert_does_not_abort_batch() {
    let provider = require_provider!();
    let lens_addr = Address::repeat_byte(0xca);

    let mut lens = Lens::new(&provider);
    lens.with_ephemeral(&lens_addr, IRevertLens::DEPLOYED_BYTECODE.clone())
        .with_call::<IERC20::nameCall>(&WETH, ())
        .with_call::<IRevertLens::testFailCall>(&lens_addr, ())
        .with_call::<IERC20::symbolCall>(&USDC, ());

    let results = lens.call().await.unwrap();

    assert_eq!(results.len(), 3);

    assert!(results[0].success);
    assert_eq!(results[0].result[0].as_str().unwrap(), "Wrapped Ether");

    assert!(!results[1].success);
    assert!(results[1].revert.is_some());
    assert!(results[1].result.is_empty());

    assert!(results[2].success);
    assert_eq!(results[2].result[0].as_str().unwrap(), "USDC");
}

/// A per-call gas cap bounds one item without starving its siblings: a call
/// given too little gas to read WETH's `name()` fails, while the calls around it
/// (uncapped) still succeed. `name()` reads storage, so 100 gas is guaranteed to
/// be insufficient.
#[tokio::test]
async fn test_per_call_gas_cap_isolates_one_item() {
    let provider = require_provider!();

    // `name()` selector.
    let name_calldata = Bytes::from_static(&[0x06, 0xfd, 0xde, 0x03]);

    let mut lens = Lens::new(&provider);
    lens.with_raw_call(&WETH, name_calldata.clone())
        .with_raw_call_gas(&WETH, name_calldata.clone(), U256::from(100u64))
        .with_raw_call(&WETH, name_calldata);

    let results = lens.call().await.unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].success, "uncapped call should succeed");
    assert!(
        !results[1].success,
        "100-gas call should run out of gas reading name()"
    );
    assert!(
        results[2].success,
        "the capped sibling must not starve the following call"
    );
}

/// Each call's gas is measured independently. A lens call that makes three
/// sub-calls internally must consume more gas than a plain storage read.
#[tokio::test]
async fn test_gas_measured_per_call() {
    let provider = require_provider!();
    let lens_addr = Address::repeat_byte(0xca);

    let mut lens = Lens::new(&provider);
    lens.with_ephemeral(&lens_addr, ITokenLens::DEPLOYED_BYTECODE.clone())
        // Simple: single storage slot read
        .with_call::<IERC20::decimalsCall>(&WETH, ())
        // Complex: three external sub-calls (name + symbol + decimals)
        .with_call::<ITokenLens::getTokenCall>(&lens_addr, (WETH,));

    let results = lens.call().await.unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));

    let simple_gas  = results[0].gas_used;
    let complex_gas = results[1].gas_used;

    assert!(simple_gas > U256::ZERO);
    assert!(
        complex_gas > simple_gas,
        "aggregated call ({complex_gas} gas) should use more than a single storage read ({simple_gas} gas)"
    );
}

/// `with_block` pins the batch to a historical block: the lens must read the
/// same WETH totalSupply as a plain eth_call pinned at that block, and not the
/// (long since changed) value at latest. Needs an archive node.
#[tokio::test]
async fn test_with_block_reads_historical_state() {
    let provider = require_provider!();
    let block = BlockId::number(20_000_000);

    let calldata = IERC20::totalSupplyCall {}.abi_encode();
    let tx = TransactionRequest::default().to(WETH).input(calldata.into());
    let raw = provider.call(tx).block(block).await.unwrap();
    let expected = IERC20::totalSupplyCall::abi_decode_returns(&raw).unwrap();

    let mut pinned = Lens::new(&provider);
    pinned.with_call::<IERC20::totalSupplyCall>(&WETH, ()).with_block(block);
    let pinned = pinned.call().await.unwrap();

    let mut latest = Lens::new(&provider);
    latest.with_call::<IERC20::totalSupplyCall>(&WETH, ());
    let latest = latest.call().await.unwrap();

    let pinned_supply = pinned[0].result[0].as_uint().unwrap().0;
    assert_eq!(pinned_supply, expected);
    assert_ne!(pinned_supply, latest[0].result[0].as_uint().unwrap().0);
}

/// `with_block_overrides` changes the environment, not the state: pinned to a
/// block, the calls must see the overridden number and timestamp.
#[tokio::test]
async fn test_with_block_overrides_sets_the_environment() {
    let provider = require_provider!();
    let mut lens = Lens::new(&provider);
    lens.with_call::<IMulticall3::getBlockNumberCall>(&MULTICALL3, ())
        .with_call::<IMulticall3::getCurrentBlockTimestampCall>(&MULTICALL3, ())
        .with_block(BlockId::number(20_000_000))
        .with_block_overrides(BlockOverrides {
            number: Some(U256::from(20_000_001u64)),
            time: Some(1_717_281_419),
            ..Default::default()
        });
    let results = lens.call().await.unwrap();
    assert_eq!(results[0].result[0].as_uint().unwrap().0, U256::from(20_000_001u64));
    assert_eq!(results[1].result[0].as_uint().unwrap().0, U256::from(1_717_281_419u64));
}

/// Runtime code returning `tx.gasprice`: GASPRICE PUSH0 MSTORE PUSH1 0x20 PUSH0 RETURN.
const GASPRICE_CODE: [u8; 7] = [0x3a, 0x5f, 0x52, 0x60, 0x20, 0x5f, 0xf3];

/// Without fees the calls see `tx.gasprice == 0`; with `with_fees` they see
/// `basefee + priority`, as a real transaction would.
#[tokio::test]
async fn test_with_fees_sets_the_gas_price() {
    let provider = require_provider!();
    let block = BlockId::number(20_000_000);
    let caller = address!("00000000000000000000000000000000000c0ffe");
    let probe = address!("00000000000000000000000000000000000ba5e0");
    let gas_price = |fees: Option<(u128, u128)>| {
        let provider = &provider;
        async move {
            let mut lens = Lens::new(provider);
            lens.with_ephemeral(&probe, Bytes::from_static(&GASPRICE_CODE))
                .with_raw_call(&probe, Bytes::new())
                .with_call::<IMulticall3::getBasefeeCall>(&MULTICALL3, ())
                .with_caller(caller)
                .with_balance(&caller, U256::from(10u128.pow(30)))
                .with_block(block);
            if let Some((max_fee, priority)) = fees {
                lens.with_fees(max_fee, priority);
            }
            let r = lens.call().await.unwrap();
            let basefee = r[1].result[0].as_uint().unwrap().0;
            (U256::from_be_slice(&r[0].raw), basefee)
        }
    };
    let (price, _) = gas_price(None).await;
    assert_eq!(price, U256::ZERO);
    let (price, basefee) = gas_price(Some((10_000_000_000_000, 3))).await;
    assert_eq!(price, basefee + U256::from(3u64));
}

/// With fees and no gas limit reth runs the call at the block gas limit; an
/// explicit limit restores the RPC gas cap. A sub-call offered 80M gas (above
/// mainnet's 60M block limit, below a 500M RPC cap) burns what it gets: it
/// loops while more than 30,000 gas are left,
/// JUMPDEST GAS PUSH2 0x7530 LT PUSH0 JUMPI STOP.
#[tokio::test]
async fn test_with_gas_limit_lifts_the_block_gas_limit() {
    let provider = require_provider!();
    let caller = address!("00000000000000000000000000000000000c0ffe");
    let burner = address!("00000000000000000000000000000000000b0b0b");
    const BURN: [u8; 9] = [0x5b, 0x5a, 0x61, 0x75, 0x30, 0x10, 0x5f, 0x57, 0x00];
    let run = |gas: Option<u64>| {
        let provider = &provider;
        async move {
            let mut lens = Lens::new(provider);
            lens.with_ephemeral(&burner, Bytes::from_static(&BURN))
                .with_raw_call_gas(&burner, Bytes::new(), U256::from(80_000_000u64))
                .with_caller(caller)
                .with_balance(&caller, U256::from(10u128.pow(30)))
                .with_fees(10_000_000_000_000, 1);
            if let Some(gas) = gas {
                lens.with_gas_limit(gas);
            }
            lens.call().await
        }
    };
    let capped = run(None).await.unwrap();
    assert!(capped[0].gas_used < U256::from(60_000_000u64));
    let lifted = run(Some(1_000_000_000)).await.unwrap();
    assert!(lifted[0].gas_used > U256::from(70_000_000u64));
}
