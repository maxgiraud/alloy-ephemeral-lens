// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

contract Proxy {

    struct CallArgument {
        address callee;
        bytes argument;
        uint256 value;
        uint256 gas;
    }

    function execute(
        CallArgument[] calldata _calls
    ) public returns (
        bytes[] memory results
    ) {
        results = new bytes[](_calls.length);
        for (uint256 i = 0; i < _calls.length; i++) {
            try this.wrapper(_calls[i]) {

            } catch (bytes memory output) {
                results[i] = output;
            }
        }
    }

    function wrapper(
        CallArgument calldata _call
    ) public {
        uint256 gasStart = gasleft();

        (bool success, bytes memory data) = _call.callee.call(_call.argument);
        revert(
            string(
                abi.encode(
                    success,
                    gasStart-gasleft(),
                    data
                )
            )
        );
    }
}

