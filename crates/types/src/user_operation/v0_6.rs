// This file is part of Rundler.
//
// Rundler is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Rundler is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Rundler.
// If not, see https://www.gnu.org/licenses/.

use ethers::{
    abi::{encode, Token},
    types::{Address, Bytes, H256, U256},
    utils::keccak256,
};
use rand::{self, RngCore};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use super::{
    GasOverheads, UserOperation as UserOperationTrait, UserOperationId, UserOperationVariant,
};
pub use crate::contracts::v0_6::shared_types::{UserOperation, UserOpsPerAggregator};
use crate::entity::{Entity, EntityType};

impl UserOperationTrait for UserOperation {
    type OptionalGas = UserOperationOptionalGas;

    fn hash(&self, entry_point: Address, chain_id: u64) -> H256 {
        keccak256(encode(&[
            Token::FixedBytes(keccak256(self.pack_for_hash()).to_vec()),
            Token::Address(entry_point),
            Token::Uint(chain_id.into()),
        ]))
        .into()
    }

    fn id(&self) -> UserOperationId {
        UserOperationId {
            sender: self.sender,
            nonce: self.nonce,
        }
    }

    fn sender(&self) -> Address {
        self.sender
    }

    fn factory(&self) -> Option<Address> {
        Self::get_address_from_field(&self.init_code)
    }

    fn paymaster(&self) -> Option<Address> {
        Self::get_address_from_field(&self.paymaster_and_data)
    }

    fn max_gas_cost(&self) -> U256 {
        let mul = if self.paymaster().is_some() { 3 } else { 1 };
        self.max_fee_per_gas
            * (self.pre_verification_gas + self.call_gas_limit + self.verification_gas_limit * mul)
    }

    fn heap_size(&self) -> usize {
        self.init_code.len()
            + self.call_data.len()
            + self.paymaster_and_data.len()
            + self.signature.len()
    }

    fn entities(&self) -> Vec<Entity> {
        EntityType::iter()
            .filter_map(|entity| {
                self.entity_address(entity)
                    .map(|address| Entity::new(entity, address))
            })
            .collect()
    }

    fn max_fee_per_gas(&self) -> U256 {
        self.max_fee_per_gas
    }

    fn max_priority_fee_per_gas(&self) -> U256 {
        self.max_priority_fee_per_gas
    }

    fn call_gas_limit(&self) -> U256 {
        self.call_gas_limit
    }

    fn pre_verification_gas(&self) -> U256 {
        self.pre_verification_gas
    }

    fn verification_gas_limit(&self) -> U256 {
        self.verification_gas_limit
    }

    fn total_verification_gas_limit(&self) -> U256 {
        let mul = if self.paymaster().is_some() { 2 } else { 1 };
        self.verification_gas_limit * mul
    }

    fn required_pre_execution_buffer(&self) -> U256 {
        self.verification_gas_limit + U256::from(5_000)
    }

    fn calc_static_pre_verification_gas(&self, include_fixed_gas_overhead: bool) -> U256 {
        let ov = GasOverheads::default();
        super::op_calldata_gas_cost(self.clone())
            + (if include_fixed_gas_overhead {
                ov.transaction_gas_overhead
            } else {
                0.into()
            })
    }

    fn clear_signature(&mut self) {
        self.signature = Bytes::default();
    }
}

impl UserOperation {
    fn get_address_from_field(data: &Bytes) -> Option<Address> {
        if data.len() < 20 {
            None
        } else {
            Some(Address::from_slice(&data[..20]))
        }
    }

    fn pack_for_hash(&self) -> Bytes {
        let hash_init_code = keccak256(self.init_code.clone());
        let hash_call_data = keccak256(self.call_data.clone());
        let hash_paymaster_and_data = keccak256(self.paymaster_and_data.clone());

        encode(&[
            Token::Address(self.sender),
            Token::Uint(self.nonce),
            Token::FixedBytes(hash_init_code.to_vec()),
            Token::FixedBytes(hash_call_data.to_vec()),
            Token::Uint(self.call_gas_limit),
            Token::Uint(self.verification_gas_limit),
            Token::Uint(self.pre_verification_gas),
            Token::Uint(self.max_fee_per_gas),
            Token::Uint(self.max_priority_fee_per_gas),
            Token::FixedBytes(hash_paymaster_and_data.to_vec()),
        ])
        .into()
    }

    fn entity_address(&self, entity: EntityType) -> Option<Address> {
        match entity {
            EntityType::Account => Some(self.sender),
            EntityType::Paymaster => self.paymaster(),
            EntityType::Factory => self.factory(),
            EntityType::Aggregator => None,
        }
    }
}

impl From<UserOperationVariant> for UserOperation {
    /// Converts a UserOperationVariant to a UserOperation 0.6
    ///
    /// # Panics
    ///
    /// Panics if the variant is not v0.6. This is for use in contexts
    /// where the variant is known to be v0.6.
    fn from(value: UserOperationVariant) -> Self {
        value.into_v0_6().expect("Expected UserOperationV0_6")
    }
}

impl From<UserOperation> for super::UserOperationVariant {
    fn from(op: UserOperation) -> Self {
        super::UserOperationVariant::V0_6(op)
    }
}

/// User operation with optional gas fields for gas estimation
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationOptionalGas {
    /// Sender (required)
    pub sender: Address,
    /// Nonce (required)
    pub nonce: U256,
    /// Init code (required)
    pub init_code: Bytes,
    /// Call data (required)
    pub call_data: Bytes,
    /// Call gas limit (optional, set to maximum if unset)
    pub call_gas_limit: Option<U256>,
    /// Verification gas limit (optional, set to maximum if unset)
    pub verification_gas_limit: Option<U256>,
    /// Pre verification gas (optional, ignored if set)
    pub pre_verification_gas: Option<U256>,
    /// Max fee per gas (optional, ignored if set)
    pub max_fee_per_gas: Option<U256>,
    /// Max priority fee per gas (optional, ignored if set)
    pub max_priority_fee_per_gas: Option<U256>,
    /// Paymaster and data (required, dummy value for gas estimation)
    pub paymaster_and_data: Bytes,
    /// Signature (required, dummy value for gas estimation)
    pub signature: Bytes,
}

impl UserOperationOptionalGas {
    /// Fill in the optional and dummy fields of the user operation with values
    /// that will cause the maximum possible calldata gas cost.
    pub fn max_fill(&self, max_call_gas: U256, max_verification_gas: U256) -> UserOperation {
        UserOperation {
            call_gas_limit: U256::MAX,
            verification_gas_limit: U256::MAX,
            pre_verification_gas: U256::MAX,
            max_fee_per_gas: U256::MAX,
            max_priority_fee_per_gas: U256::MAX,
            signature: vec![255_u8; self.signature.len()].into(),
            paymaster_and_data: vec![255_u8; self.paymaster_and_data.len()].into(),
            ..self
                .clone()
                .into_user_operation(max_call_gas, max_verification_gas)
        }
    }

    /// Fill in the optional and dummy fields of the user operation with random values.
    ///
    /// When estimating pre-verification gas, specifically on networks that use
    /// compression algorithms on their data that they post to their data availability
    /// layer (like Arbitrum), it is important to make sure that the data that is
    /// random such that it compresses to a representative size.
    //
    /// Note that this will slightly overestimate the calldata gas needed as it uses
    /// the worst case scenario for the unknown gas values and paymaster_and_data.
    pub fn random_fill(&self, max_call_gas: U256, max_verification_gas: U256) -> UserOperation {
        UserOperation {
            call_gas_limit: U256::from_big_endian(&Self::random_bytes(4)), // 30M max
            verification_gas_limit: U256::from_big_endian(&Self::random_bytes(4)), // 30M max
            pre_verification_gas: U256::from_big_endian(&Self::random_bytes(4)), // 30M max
            max_fee_per_gas: U256::from_big_endian(&Self::random_bytes(8)), // 2^64 max
            max_priority_fee_per_gas: U256::from_big_endian(&Self::random_bytes(8)), // 2^64 max
            signature: Self::random_bytes(self.signature.len()),
            paymaster_and_data: Self::random_bytes(self.paymaster_and_data.len()),
            ..self
                .clone()
                .into_user_operation(max_call_gas, max_verification_gas)
        }
    }

    /// Convert into a full user operation.
    /// Fill in the optional fields of the user operation with default values if unset
    pub fn into_user_operation(
        self,
        max_call_gas: U256,
        max_verification_gas: U256,
    ) -> UserOperation {
        UserOperation {
            sender: self.sender,
            nonce: self.nonce,
            init_code: self.init_code,
            call_data: self.call_data,
            paymaster_and_data: self.paymaster_and_data,
            signature: self.signature,
            // If unset, default these to gas limits from settings
            // Cap their values to the gas limits from settings
            verification_gas_limit: self
                .verification_gas_limit
                .unwrap_or(max_verification_gas)
                .min(max_verification_gas),
            call_gas_limit: self
                .call_gas_limit
                .unwrap_or(max_call_gas)
                .min(max_call_gas),
            // These aren't used in gas estimation, set to if unset 0 so that there are no payment attempts during gas estimation
            pre_verification_gas: self.pre_verification_gas.unwrap_or_default(),
            max_fee_per_gas: self.max_fee_per_gas.unwrap_or_default(),
            max_priority_fee_per_gas: self.max_priority_fee_per_gas.unwrap_or_default(),
        }
    }

    fn random_bytes(len: usize) -> Bytes {
        let mut bytes = vec![0_u8; len];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes.into()
    }
}

impl From<super::UserOperationOptionalGas> for UserOperationOptionalGas {
    fn from(op: super::UserOperationOptionalGas) -> Self {
        match op {
            super::UserOperationOptionalGas::V0_6(op) => op,
            _ => panic!("Expected UserOperationOptionalGasV0_6"),
        }
    }
}

#[cfg(test)]
mod tests {

    use ethers::types::{Bytes, U256};

    use super::*;

    #[test]
    fn test_hash_zeroed() {
        // Testing a user operation hash against the hash generated by the
        // entrypoint contract getUserOpHash() function with entrypoint address
        // at 0x66a15edcc3b50a663e72f1457ffd49b9ae284ddc and chain ID 1337.
        //
        // UserOperation = {
        //     sender: '0x0000000000000000000000000000000000000000',
        //     nonce: 0,
        //     initCode: '0x',
        //     callData: '0x',
        //     callGasLimit: 0,
        //     verificationGasLimit: 0,
        //     preVerificationGas: 0,
        //     maxFeePerGas: 0,
        //     maxPriorityFeePerGas: 0,
        //     paymasterAndData: '0x',
        //     signature: '0x',
        //   }
        //
        // Hash: 0xdca97c3b49558ab360659f6ead939773be8bf26631e61bb17045bb70dc983b2d
        let operation = UserOperation {
            sender: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            nonce: U256::zero(),
            init_code: Bytes::default(),
            call_data: Bytes::default(),
            call_gas_limit: U256::zero(),
            verification_gas_limit: U256::zero(),
            pre_verification_gas: U256::zero(),
            max_fee_per_gas: U256::zero(),
            max_priority_fee_per_gas: U256::zero(),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };
        let entry_point = "0x66a15edcc3b50a663e72f1457ffd49b9ae284ddc"
            .parse()
            .unwrap();
        let chain_id = 1337;
        let hash = operation.hash(entry_point, chain_id);
        assert_eq!(
            hash,
            "0xdca97c3b49558ab360659f6ead939773be8bf26631e61bb17045bb70dc983b2d"
                .parse()
                .unwrap()
        );
    }

    #[test]
    fn test_hash() {
        // Testing a user operation hash against the hash generated by the
        // entrypoint contract getUserOpHash() function with entrypoint address
        // at 0x66a15edcc3b50a663e72f1457ffd49b9ae284ddc and chain ID 1337.
        //
        // UserOperation = {
        //     sender: '0x1306b01bc3e4ad202612d3843387e94737673f53',
        //     nonce: 8942,
        //     initCode: '0x6942069420694206942069420694206942069420',
        //     callData: '0x0000000000000000000000000000000000000000080085',
        //     callGasLimit: 10000,
        //     verificationGasLimit: 100000,
        //     preVerificationGas: 100,
        //     maxFeePerGas: 99999,
        //     maxPriorityFeePerGas: 9999999,
        //     paymasterAndData:
        //       '0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
        //     signature:
        //       '0xda0929f527cded8d0a1eaf2e8861d7f7e2d8160b7b13942f99dd367df4473a',
        //   }
        //
        // Hash: 0x484add9e4d8c3172d11b5feb6a3cc712280e176d278027cfa02ee396eb28afa1
        let operation = UserOperation {
            sender: "0x1306b01bc3e4ad202612d3843387e94737673f53"
                .parse()
                .unwrap(),
            nonce: 8942.into(),
            init_code: "0x6942069420694206942069420694206942069420"
                .parse()
                .unwrap(),
            call_data: "0x0000000000000000000000000000000000000000080085"
                .parse()
                .unwrap(),
            call_gas_limit: 10000.into(),
            verification_gas_limit: 100000.into(),
            pre_verification_gas: 100.into(),
            max_fee_per_gas: 99999.into(),
            max_priority_fee_per_gas: 9999999.into(),
            paymaster_and_data:
                "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .parse()
                    .unwrap(),
            signature: "0xda0929f527cded8d0a1eaf2e8861d7f7e2d8160b7b13942f99dd367df4473a"
                .parse()
                .unwrap(),
        };
        let entry_point = "0x66a15edcc3b50a663e72f1457ffd49b9ae284ddc"
            .parse()
            .unwrap();
        let chain_id = 1337;
        let hash = operation.hash(entry_point, chain_id);
        assert_eq!(
            hash,
            "0x484add9e4d8c3172d11b5feb6a3cc712280e176d278027cfa02ee396eb28afa1"
                .parse()
                .unwrap()
        );
    }

    #[test]
    fn test_get_address_from_field() {
        let paymaster_and_data: Bytes =
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .parse()
                .unwrap();
        let address = UserOperation::get_address_from_field(&paymaster_and_data).unwrap();
        assert_eq!(
            address,
            "0x0123456789abcdef0123456789abcdef01234567"
                .parse()
                .unwrap()
        );
    }
}
