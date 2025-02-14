// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use serde_json::json;

mod bytes;
mod parse;
mod serialize;

use crate::{ArrayType, Identifier, LiteralType};
use snarkvm_console_network::prelude::*;

/// A `PlaintextType` defines the type parameter for a literal, struct, or array.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PlaintextType<N: Network> {
    /// A literal type contains its type name.
    /// The format of the type is `<type_name>`.
    Literal(LiteralType),
    /// An struct type contains its identifier.
    /// The format of the type is `<identifier>`.
    Struct(Identifier<N>),
    /// An array type contains its element type and length.
    /// The format of the type is `[<element_type>; <length>]`.
    Array(ArrayType<N>),
}

/// ** Vanguard JSON serialization helper ** ///
impl<N: Network> PlaintextType<N> {
    pub fn to_json(&self) -> serde_json::Value {
        let j_vtype = match self {
            // Prints the literal, i.e. field
            Self::Literal(_) => "Literal",
            // Prints the struct, i.e. signature
            Self::Struct(_) => "Struct",
            // Prints the array type, i.e. [field; 2u32]
            Self::Array(_) => "Array",
        };

        let j_value = match self {
            // Prints the literal, i.e. field
            Self::Literal(literal) => literal.to_json(),
            // Prints the struct, i.e. signature
            Self::Struct(struct_) => struct_.to_json(),
            // Prints the array type, i.e. [field; 2u32]
            Self::Array(array) => array.to_json(),
        };

        json!({
            "type": "PlaintextType",
            "vtype": j_vtype,
            "value": j_value,
        })
    }
}

impl<N: Network> From<LiteralType> for PlaintextType<N> {
    /// Initializes a plaintext type from a literal type.
    fn from(literal: LiteralType) -> Self {
        PlaintextType::Literal(literal)
    }
}

impl<N: Network> From<Identifier<N>> for PlaintextType<N> {
    /// Initializes a plaintext type from a struct type.
    fn from(struct_: Identifier<N>) -> Self {
        PlaintextType::Struct(struct_)
    }
}

impl<N: Network> From<ArrayType<N>> for PlaintextType<N> {
    /// Initializes a plaintext type from an array type.
    fn from(array: ArrayType<N>) -> Self {
        PlaintextType::Array(array)
    }
}
