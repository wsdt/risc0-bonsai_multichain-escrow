// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

pragma solidity ^0.8.16;

import {IBonsaiProxy} from "./IBonsaiProxy.sol";
import {BonsaiApp} from "./BonsaiApp.sol";

/// @title A starter application using Bonsai through the on-chain proxy.
/// @dev This contract demonstrates one pattern for offloading the computation of an expensive
//       or difficult to implement function to a RISC Zero guest running on Bonsai.
contract HelloBonsai is BonsaiApp {
    // Cache of the results calculated by our guest program in Bonsai.
    mapping(uint256 => uint256) public fibonnaci_cache;

    // Initialize the contract, binding it to a specified Bonsai proxy and RISC Zero guest image.
    constructor(
        IBonsaiProxy _bonsai_proxy,
        bytes32 _image_id
    ) BonsaiApp(_bonsai_proxy, _image_id) {} // solhint-disable-line no-empty-blocks

    event CalculateFibonacciCallback(uint256 indexed n, uint256 result);

    /// @notice Returns nth number in the Fibonacci sequence.
    /// @dev The sequence is defined as 1, 1, 2, 3, 5 ... with fibonnacci(0) == 1.
    ///      Only precomputed results can be returned. Call calculate_fibonacci(n) to precompute.
    function fibonacci(uint256 n) external view returns (uint256) {
        uint256 result = fibonnaci_cache[n];
        require(result != 0, "value not available in cache");
        return result;
    }

    /// @notice Sends a request to Bonsai to have have the nth Fibonacci number calculated.
    /// @dev This function sends the request to Bonsai through the on-chain proxy.
    ///      The request will trigger Bonsai to run the specified RISC Zero guest program with
    ///      the given input and asynchronously return the verified results via the callback below.
    function calculate_fibonacci(uint256 n) external {
        submit_bonsai_request(abi.encode(n));
    }

    /// @notice Callback function logic for processing verified journals from Bonsai.
    function bonsai_callback(bytes memory journal) internal override {
        (uint256 n, uint256 result) = abi.decode(journal, (uint256, uint256));

        emit CalculateFibonacciCallback(n, result);
        fibonnaci_cache[n] = result;
    }
}
