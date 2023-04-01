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

/// @dev This contract demonstrates one pattern for offloading the computation of an expensive
//       or difficult to implement function to a RISC Zero guest running on Bonsai.
contract L3L1Escrow is BonsaiApp {
    // V1: One-way bridge for ValidityRollups (easily to deploy other way round)
    // 1. user A deposits funds on chainA and provides address of creditor on chainB
    // 2. user B send funds to user A on chainB (regular transfer)
    // 3. Escrow checks if user B has sent funds to user A on chainB
    // 4. On success release funds from Escrow on chainA to user B
    struct Deposit {
        // who should get it
        address creditor;
        uint256 amount;
        uint256 earliestTime;
    }
    mapping(address => Deposit) public deposits;

    // Initialize the contract, binding it to a specified Bonsai proxy and RISC Zero guest image.
    constructor(
        IBonsaiProxy _bonsai_proxy,
        bytes32 _image_id
    ) BonsaiApp(_bonsai_proxy, _image_id) {} // solhint-disable-line no-empty-blocks

    event CrosschainPaymentReceived(address indexed sender, address indexed recipient, uint256 amount);

    /// @notice Sends a request to Bonsai to have have the nth Fibonacci number calculated.
    /// @dev This function sends the request to Bonsai through the on-chain proxy.
    ///      The request will trigger Bonsai to run the specified RISC Zero guest program with
    ///      the given input and asynchronously return the verified results via the callback below.
    function checkPaymentStatus(bytes32 txHash) external {
        require(deposits[msg.sender].amount > 0, "No deposit found");
        require(deposits[msg.sender].earliestTime < block.timestamp, "Too early");

        // working example for eth: 0x671a3b40ecb7d51b209e68392df2d38c098aae03febd3a88be0f1fa77725bbd7
        Deposit memory deposit = deposits[msg.sender];
        submit_bonsai_request(serialize(abi.encode(txHash, msg.sender, deposit.creditor, deposit.amount)));
    }

    function deposit(address creditor, uint256 amount) payable external {
        require(msg.value == amount, "Amount invalid");
        deposits[msg.sender] = Deposit(creditor, amount, block.number); // 1 ETH for 1 ETH
    }

    /// @notice Callback function logic for processing verified journals from Bonsai.
    function bonsai_callback(bytes memory journal) internal override {
        (bool success, address depositor) = abi.decode(journal, (bool, address));
        require (success, "Bonsai error");
        Deposit memory deposit = deposits[msg.sender];
        require(deposit.amount > 0, "No deposit found");

        payable(deposit.creditor).transfer(deposit.amount);

        emit CrosschainPaymentReceived(depositor, deposit.creditor, deposit.amount);
        delete deposits[msg.sender];
    }

    function serialize(bytes memory input) public pure returns (bytes memory) {
        uint256 length = input.length;

        bytes memory result = new bytes(0);
        result = bytes.concat(bytes.concat(bytes4(reverse(uint32(length)))), input);
        return result;
    }

    // copied from https://ethereum.stackexchange.com/questions/83626/how-to-reverse-byte-order-in-uint256-or-bytes32
    function reverse(uint32 input) internal pure returns (uint32 v) {
        v = input;

        // swap bytes
        v = ((v & 0xFF00FF00) >> 8) |
        ((v & 0x00FF00FF) << 8);

        // swap 2-byte long pairs
        v = (v >> 16) | (v << 16);
    }
}
