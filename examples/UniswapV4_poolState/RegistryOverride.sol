// SPDX-License-Identifier: GPL-2.0-or-later
pragma solidity >=0.8.0;

type PoolId is bytes32;
type Slot0 is bytes32;

library Slot0Library {
    uint8 internal constant TICK_OFFSET = 160;
    uint160 internal constant MASK_160_BITS = 0x00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
    
    function tick(Slot0 _packed) internal pure returns (int24 _tick) {
        assembly ("memory-safe") {
            _tick := signextend(2, shr(TICK_OFFSET, _packed))
        }
    }

    function sqrtPriceX96(Slot0 _packed) internal pure returns (uint160 _sqrtPriceX96) {
        assembly ("memory-safe") {
            _sqrtPriceX96 := and(MASK_160_BITS, _packed)
        }
    }
}

contract RegistryOverride {
    using Slot0Library for Slot0;

    struct PoolState {
        uint128 liquidity;
        uint160 sqrtPriceX96;
        int24 tick;
    }

    struct TickInfo {
        // the total position liquidity that references this tick
        uint128 liquidityGross;
        // amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left),
        int128 liquidityNet;
        // fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
        // only has relative meaning, not absolute — the value depends on when the tick is initialized
        uint256 feeGrowthOutside0X128;
        uint256 feeGrowthOutside1X128;
    }

    struct State {
        Slot0 slot0;
        uint256 feeGrowthGlobal0X128;
        uint256 feeGrowthGlobal1X128;
        uint128 liquidity;
        mapping(int24 tick => TickInfo) ticks;
        mapping(int16 wordPos => uint256) tickBitmap;
        // mapping(bytes32 positionKey => Position.State) positions;
    }

    // Slot padding
    // Number of slot found by trial and error
    bytes32 _slot0;
    bytes32 _slot1;
    bytes32 _slot2;
    bytes32 _slot3;
    bytes32 _slot4;
    bytes32 _slot5;

    mapping(bytes32 id => State) internal _pools;


    function getState(
        bytes32 poolId
    ) public view returns(
        PoolState memory poolState
    ) {
        State storage state = _pools[poolId];

        uint160 sqrtPriceX96 = state.slot0.sqrtPriceX96();

        int24 tick = state.slot0.tick();

        poolState.liquidity = state.liquidity;
        poolState.sqrtPriceX96 = sqrtPriceX96;
        poolState.tick = tick;
    }
}