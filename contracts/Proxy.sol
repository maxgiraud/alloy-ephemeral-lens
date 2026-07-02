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

        // A zero `gas` means "forward all remaining gas" (the historical
        // behaviour); a non-zero value caps this item so one gas-heavy call
        // cannot starve its siblings in the same batch.
        uint256 gasLimit = _call.gas == 0 ? gasleft() : _call.gas;
        (bool success, bytes memory data) = _call.callee.call{gas: gasLimit}(_call.argument);
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

