use alloy::{primitives::{address, Address}, providers::{ProviderBuilder, WsConnect}, sol};
use alloy_ephemeral_lens::Lens;

sol! {
    #[sol(rpc, abi, deployed_bytecode="608060405234801561000f575f5ffd5b506004361061003f575f3560e01c806304d91c6a146100435780635977043814610061578063f8a8fd6d14610094575b5f5ffd5b61004b6100b2565b60405161005891906102fe565b60405180910390f35b61007b60048036038101906100769190610389565b6100ef565b60405161008b94939291906103de565b60405180910390f35b61009c610251565b6040516100a991906102fe565b60405180910390f35b60606040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016100e690610479565b60405180910390fd5b5f6060805f848573ffffffffffffffffffffffffffffffffffffffff166306fdde036040518163ffffffff1660e01b81526004015f60405180830381865afa15801561013d573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f8201168201806040525081019061016591906105b5565b8673ffffffffffffffffffffffffffffffffffffffff166395d89b416040518163ffffffff1660e01b81526004015f60405180830381865afa1580156101ad573d5f5f3e3d5ffd5b505050506040513d5f823e3d601f19601f820116820180604052508101906101d591906105b5565b8773ffffffffffffffffffffffffffffffffffffffff1663313ce5676040518163ffffffff1660e01b8152600401602060405180830381865afa15801561021e573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906102429190610626565b93509350935093509193509193565b60606040518060400160405280600681526020017f636f75636f750000000000000000000000000000000000000000000000000000815250905090565b5f81519050919050565b5f82825260208201905092915050565b8281835e5f83830152505050565b5f601f19601f8301169050919050565b5f6102d08261028e565b6102da8185610298565b93506102ea8185602086016102a8565b6102f3816102b6565b840191505092915050565b5f6020820190508181035f83015261031681846102c6565b905092915050565b5f604051905090565b5f5ffd5b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6103588261032f565b9050919050565b6103688161034e565b8114610372575f5ffd5b50565b5f813590506103838161035f565b92915050565b5f6020828403121561039e5761039d610327565b5b5f6103ab84828501610375565b91505092915050565b6103bd8161034e565b82525050565b5f60ff82169050919050565b6103d8816103c3565b82525050565b5f6080820190506103f15f8301876103b4565b818103602083015261040381866102c6565b9050818103604083015261041781856102c6565b905061042660608301846103cf565b95945050505050565b7f41696520636f75702064757220706f7572206775696c6c61756d6500000000005f82015250565b5f610463601b83610298565b915061046e8261042f565b602082019050919050565b5f6020820190508181035f83015261049081610457565b9050919050565b5f5ffd5b5f5ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b6104d5826102b6565b810181811067ffffffffffffffff821117156104f4576104f361049f565b5b80604052505050565b5f61050661031e565b905061051282826104cc565b919050565b5f67ffffffffffffffff8211156105315761053061049f565b5b61053a826102b6565b9050602081019050919050565b5f61055961055484610517565b6104fd565b9050828152602081018484840111156105755761057461049b565b5b6105808482856102a8565b509392505050565b5f82601f83011261059c5761059b610497565b5b81516105ac848260208601610547565b91505092915050565b5f602082840312156105ca576105c9610327565b5b5f82015167ffffffffffffffff8111156105e7576105e661032b565b5b6105f384828501610588565b91505092915050565b610605816103c3565b811461060f575f5ffd5b50565b5f81519050610620816105fc565b92915050565b5f6020828403121561063b5761063a610327565b5b5f61064884828501610612565b9150509291505056fea2646970667358221220ed60340e334891abf19b2a56f5de2ec999aa1bf63e723ba46959bb975cd0050a64736f6c634300081b0033")]
    interface ITokenLens {
        #[sol(abi)]
        function getToken(address) external view returns (address,string,string,uint8);
        #[sol(abi)]
        function test() external view returns (string memory);
        #[sol(abi)]
        function testFail() external view returns (string memory);
    }
}

#[tokio::main]
async fn main() {

    let rpc_url = "";
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await.unwrap();

    let lens_address = Address::repeat_byte(0xca);

    let mut lens= Lens::new(&provider);
    lens
        .with_ephemeral(&lens_address, ITokenLens::DEPLOYED_BYTECODE.clone())
        .with_call::<ITokenLens::getTokenCall>(&lens_address, (address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),))
        .with_call::<ITokenLens::getTokenCall>(&lens_address, (address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),))
        .with_call::<ITokenLens::testCall>(&lens_address, ())
        .with_call::<ITokenLens::testFailCall>(&lens_address, ())
        ;

    let result = lens.call().await;

    dbg!(result);
}