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
use std::collections::HashMap;

mod entry_type;
pub use entry_type::EntryType;

mod helpers;
use helpers::PublicOrPrivate;

mod bytes;
mod parse;
mod serialize;

use crate::Identifier;
use snarkvm_console_network::prelude::*;

use indexmap::IndexMap;

/// The declared layout for program data.
#[derive(Clone, PartialEq, Eq)]
pub struct RecordType<N: Network> {
    /// The name of the record type.
    name: Identifier<N>,
    /// The visibility for the owner of the program record.
    owner: PublicOrPrivate,
    /// The name and value type for the entries in data.
    entries: IndexMap<Identifier<N>, EntryType<N>>,
}

/// ** Vanguard JSON serialization helper ** ///
impl<N: Network> RecordType<N> {
    pub fn to_json(&self) -> serde_json::Value {
        let mut j_entries: HashMap<String, serde_json::Value> = HashMap::new();
        for (key, val) in &self.entries {
            j_entries.insert(key.to_key(), val.to_json());
        }

        json!({
            "type": "RecordType",
            "name": self.name.to_json(),
            "owner": self.owner.to_json(),
            "entries": j_entries,
        })
    }
}

impl<N: Network> RecordType<N> {
    /// Returns the name of the record type.
    pub const fn name(&self) -> &Identifier<N> {
        &self.name
    }

    /// Returns the visibility for the owner of the program record.
    pub const fn owner(&self) -> PublicOrPrivate {
        self.owner
    }

    /// Returns the entries of the record type.
    pub const fn entries(&self) -> &IndexMap<Identifier<N>, EntryType<N>> {
        &self.entries
    }
}

impl<N: Network> TypeName for RecordType<N> {
    /// Returns the type name.
    fn type_name() -> &'static str {
        "record"
    }
}
