/*
    An example of a Lens that retrieve UniswapV4 pool state.

    UniswapV4 pool state are stored in a mapping of the pool manager :
    ```sol
        mapping(PoolId id => Pool.State) internal _pools;
    ```

    The pool state is represented by a complex struct :
    ```sol
        struct State {
            Slot0 slot0;
            uint256 feeGrowthGlobal0X128;
            uint256 feeGrowthGlobal1X128;
            uint128 liquidity;
            mapping(int24 tick => TickInfo) ticks;
            mapping(int16 wordPos => uint256) tickBitmap;
            mapping(bytes32 positionKey => Position.State) positions;
        }
    ```

    One problem here is that this mapping of pool state is private.
    We can still leverage ephemeral lens to get this data.

    For this we override the `code` at pool manager address with our own lens `RegistryOverride.sol`

    Then we can querry multiple pool states in a signe `eth_call`
*/

use std::env;

use alloy::{hex::FromHex, primitives::{address, FixedBytes}, providers::{ProviderBuilder, WsConnect}, sol};
use alloy_ephemeral_lens::Lens;

// Registry Override
sol! {
    // `$ solc --bin-runtime examples/UniswapV4_poolState/RegistryOverride.sol`
    #[sol(deployed_bytecode="608060405234801561000f575f5ffd5b5060043610610029575f3560e01c806309648a9d1461002d575b5f5ffd5b610047600480360381019061004291906101e5565b61005d565b60405161005491906102c3565b60405180910390f35b610065610165565b5f60065f8481526020019081526020015f2090505f610086825f0154610137565b90505f610095835f0154610156565b9050826003015f9054906101000a90046fffffffffffffffffffffffffffffffff16845f01906fffffffffffffffffffffffffffffffff1690816fffffffffffffffffffffffffffffffff168152505081846020019073ffffffffffffffffffffffffffffffffffffffff16908173ffffffffffffffffffffffffffffffffffffffff168152505080846040019060020b908160020b81525050505050919050565b5f8173ffffffffffffffffffffffffffffffffffffffff169050919050565b5f8160a01c60020b9050919050565b60405180606001604052805f6fffffffffffffffffffffffffffffffff1681526020015f73ffffffffffffffffffffffffffffffffffffffff1681526020015f60020b81525090565b5f5ffd5b5f819050919050565b6101c4816101b2565b81146101ce575f5ffd5b50565b5f813590506101df816101bb565b92915050565b5f602082840312156101fa576101f96101ae565b5b5f610207848285016101d1565b91505092915050565b5f6fffffffffffffffffffffffffffffffff82169050919050565b61023481610210565b82525050565b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b6102628161023a565b82525050565b5f8160020b9050919050565b61027d81610268565b82525050565b606082015f8201516102975f85018261022b565b5060208201516102aa6020850182610259565b5060408201516102bd6040850182610274565b50505050565b5f6060820190506102d65f830184610283565b9291505056fea26469706673582212200d1db85af0bec75f830f338295967473fc73bc9886b378789d6776a5b57bfc5b64736f6c634300081b0033")]
    interface IRegistryOverride {

        struct PoolState {
            uint128 liquidity;
            uint160 sqrtPriceX96;
            int24 tick;
        }

        #[sol(abi)]
        function getState(bytes32 poolId) public view returns(PoolState memory);
    }
}

#[tokio::main]
async fn main() {

    let rpc_url = env::var("RPC_URL").unwrap();
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await.unwrap();

    let pool_manager_address = address!("0x000000000004444c5dc75cB358380D2e3dE08A90");

    let result= Lens::new(&provider)
        // The registry override deployed at the pool manager address
        .with_ephemeral(&pool_manager_address, IRegistryOverride::DEPLOYED_BYTECODE.clone())
        .with_call::<IRegistryOverride::getStateCall>(&pool_manager_address, (FixedBytes::from_hex("0x21c67e77068de97969ba93d4aab21826d33ca12bb9f565d8496e8fda8a82ca27").unwrap(),))
        .with_call::<IRegistryOverride::getStateCall>(&pool_manager_address, (FixedBytes::from_hex("0xccc8eec61db9eac7106cc110b6834c2f9539ea7dd8df139e57587bcb1a701611").unwrap(),))
        .call().await;

    println!("{:?}", result);
}