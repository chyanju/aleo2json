// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

mod string;

use crate::{
    console::{
        account::{Address, PrivateKey},
        network::prelude::*,
        program::{Identifier, ProgramID, Value},
    },
    ledger::{Header, Transaction, Transactions},
    vm::VM,
};
use snarkvm_compiler::Program;

#[derive(Clone, PartialEq, Eq)]
pub struct Block<N: Network> {
    /// The hash of this block.
    block_hash: N::BlockHash,
    /// The hash of the previous block.
    previous_hash: N::BlockHash,
    /// The header of the block.
    header: Header<N>,
    /// The transactions in the block.
    transactions: Transactions<N>,
}

impl<N: Network> Block<N> {
    /// Initializes a new block from a given previous hash, header, and transactions list.
    pub fn from(previous_hash: N::BlockHash, header: Header<N>, transactions: Transactions<N>) -> Result<Self> {
        // Ensure the block is not empty.
        ensure!(!transactions.is_empty(), "Cannot create block with no transactions");
        // Compute the block hash.
        let block_hash =
            N::hash_bhp1024(&[previous_hash.to_bits_le(), header.to_root()?.to_bits_le()].concat())?.into();
        // Construct the block.
        Ok(Self { block_hash, previous_hash, header, transactions })
    }

    /// Initializes a new genesis block.
    pub fn genesis<R: Rng + CryptoRng>(vm: &mut VM<N>, private_key: &PrivateKey<N>, rng: &mut R) -> Result<Self> {
        // Initialize the genesis program.
        let genesis = Program::genesis()?;
        // Deploy the genesis program.
        let deploy = vm.deploy(&genesis, rng)?;
        // Add the genesis program.
        vm.on_deploy(&deploy)?;

        // Prepare the caller.
        let caller = Address::try_from(private_key)?;
        // Prepare the function name.
        let function_name = FromStr::from_str("start")?;
        // Prepare the function inputs.
        let inputs = [Value::from_str(&caller.to_string())?, Value::from_str("1_100_000_000_000_000_u64")?];
        // Authorize the call to start.
        let authorization = vm.authorize(private_key, genesis.id(), function_name, &inputs, rng)?;
        // Execute the genesis program.
        let (_, execution) = vm.execute(authorization, rng)?;

        // Prepare the components.
        let header = Header::genesis();
        let transactions = Transactions::from(&[deploy, execution])?;
        let previous_hash = N::BlockHash::default();

        // Construct the block.
        let block = Self::from(previous_hash, header, transactions)?;
        // Ensure the block is valid genesis block.
        match block.is_genesis() {
            true => Ok(block),
            false => bail!("Failed to initialize a genesis block"),
        }
    }

    /// Returns `true` if the block is well-formed.
    pub fn verify(&self, vm: &VM<N>) -> bool {
        // If the block is the genesis block, check that it is valid.
        if self.header.height() == 0 && !self.is_genesis() {
            warn!("Invalid genesis block");
            return false;
        }

        // Ensure the block header is valid.
        if !self.header.is_valid() {
            warn!("Invalid block header: {:?}", self.header);
            return false;
        }

        // Compute the Merkle root of the block header.
        let header_root = match self.header.to_root() {
            Ok(root) => root,
            Err(error) => {
                warn!("Failed to compute the Merkle root of the block header: {error}");
                return false;
            }
        };

        // Check the block hash.
        match N::hash_bhp1024(&[self.previous_hash.to_bits_le(), header_root.to_bits_le()].concat()) {
            Ok(candidate_hash) => {
                // Ensure the block hash matches the one in the block.
                if candidate_hash != *self.block_hash {
                    warn!("Block ({}) has an incorrect block hash.", self.block_hash);
                    return false;
                }
            }
            Err(error) => {
                warn!("Unable to compute block hash for block ({}): {error}", self.block_hash);
                return false;
            }
        };

        // Ensure the transactions are valid.
        if !self.transactions.verify(vm) {
            warn!("Block contains invalid transactions: {:?}", self);
            return false;
        }

        true
    }

    /// Returns `true` if the block is a genesis block.
    pub fn is_genesis(&self) -> bool {
        // Ensure the previous block hash is zero.
        self.previous_hash == N::BlockHash::default()
            // Ensure the header is a genesis block header.
            && self.header.is_genesis()
            // Ensure there is one transaction in the genesis block.
            && self.transactions.len() == 1
    }

    /// Returns the block hash.
    pub const fn hash(&self) -> N::BlockHash {
        self.block_hash
    }

    /// Returns the previous block hash.
    pub const fn previous_hash(&self) -> N::BlockHash {
        self.previous_hash
    }

    /// Returns the block header.
    pub const fn header(&self) -> &Header<N> {
        &self.header
    }

    /// Returns the transactions in the block.
    pub const fn transactions(&self) -> &Transactions<N> {
        &self.transactions
    }
}

impl<N: Network> Block<N> {
    /// Returns an iterator over all transactions in `self` that are deployments.
    pub fn deployments(&self) -> impl '_ + Iterator<Item = &Transaction<N>> {
        self.transactions.deployments()
    }

    /// Returns an iterator over all transactions in `self` that are executions.
    pub fn executions(&self) -> impl '_ + Iterator<Item = &Transaction<N>> {
        self.transactions.executions()
    }
}

impl<N: Network> FromBytes for Block<N> {
    /// Reads the block from the buffer.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u16::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 0 {
            return Err(error("Invalid block version"));
        }

        // Read the block.
        let block_hash: N::BlockHash = FromBytes::read_le(&mut reader)?;
        let previous_hash = FromBytes::read_le(&mut reader)?;
        let header = FromBytes::read_le(&mut reader)?;
        let transactions = FromBytes::read_le(&mut reader)?;

        // Construct the block.
        let block = Self::from(previous_hash, header, transactions).map_err(|e| error(e.to_string()))?;
        // Ensure the block hash matches.
        match block_hash == block.hash() {
            true => Ok(block),
            false => Err(error("Mismatching block hash, possible data corruption")),
        }
    }
}

impl<N: Network> ToBytes for Block<N> {
    /// Writes the block to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        0u16.write_le(&mut writer)?;

        // Write the block.
        self.block_hash.write_le(&mut writer)?;
        self.previous_hash.write_le(&mut writer)?;
        self.header.write_le(&mut writer)?;
        self.transactions.write_le(&mut writer)
    }
}

impl<N: Network> Serialize for Block<N> {
    /// Serializes the block to a JSON-string or buffer.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut block = serializer.serialize_struct("Block", 4)?;
                block.serialize_field("block_hash", &self.block_hash)?;
                block.serialize_field("previous_hash", &self.previous_hash)?;
                block.serialize_field("header", &self.header)?;
                block.serialize_field("transactions", &self.transactions)?;
                block.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Block<N> {
    /// Deserializes the block from a JSON-string or buffer.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let block = serde_json::Value::deserialize(deserializer)?;
                let block_hash: N::BlockHash =
                    serde_json::from_value(block["block_hash"].clone()).map_err(de::Error::custom)?;

                // Recover the block.
                let block = Self::from(
                    serde_json::from_value(block["previous_hash"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["header"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["transactions"].clone()).map_err(de::Error::custom)?,
                )
                .map_err(de::Error::custom)?;

                // Ensure the block hash matches.
                match block_hash == block.hash() {
                    true => Ok(block),
                    false => Err(error("Mismatching block hash, possible data corruption")).map_err(de::Error::custom),
                }
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "block"),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     use snarkvm::prelude::Testnet3;
//
//     type CurrentNetwork = Testnet3;
//     type A = snarkvm::circuit::AleoV0;
//
//     #[test]
//     fn test_block_serde_json() {
//         let block = Block::<CurrentNetwork>::genesis::<A>().unwrap();
//
//         // Serialize
//         let expected_string = block.to_string();
//         let candidate_string = serde_json::to_string(&block).unwrap();
//         assert_eq!(3057, candidate_string.len(), "Update me if serialization has changed");
//         assert_eq!(expected_string, candidate_string);
//
//         // Deserialize
//         assert_eq!(block, Block::from_str(&candidate_string).unwrap());
//         assert_eq!(block, serde_json::from_str(&candidate_string).unwrap());
//     }
//
//     #[test]
//     fn test_block_bincode() {
//         let block = Block::<CurrentNetwork>::genesis::<A>().unwrap();
//
//         // Serialize
//         let expected_bytes = block.to_bytes_le().unwrap();
//         let candidate_bytes = bincode::serialize(&block).unwrap();
//         assert_eq!(1532, expected_bytes.len(), "Update me if serialization has changed");
//         // TODO (howardwu): Serialization - Handle the inconsistency between ToBytes and Serialize (off by a length encoding).
//         assert_eq!(&expected_bytes[..], &candidate_bytes[8..]);
//
//         // Deserialize
//         assert_eq!(block, Block::read_le(&expected_bytes[..]).unwrap());
//         assert_eq!(block, bincode::deserialize(&candidate_bytes[..]).unwrap());
//     }
//
//     #[test]
//     fn test_block_genesis() {
//         let block = Block::<CurrentNetwork>::genesis::<A>().unwrap();
//         assert!(block.is_genesis());
//     }
// }
