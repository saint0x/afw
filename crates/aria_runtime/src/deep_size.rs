//!
//! This module provides newtype wrappers for external types to allow the implementation
//! of the `DeepSizeOf` trait, satisfying Rust's orphan rule. It also provides necessary
//! trait implementations (`Serialize`, `Deserialize`, `Deref`, etc.) to make these
//! wrappers ergonomic to use.
//!

use deepsize::DeepSizeOf;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

// --- Newtype Wrappers ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeepUuid(pub Uuid);

#[derive(Debug, Clone, PartialEq)]
pub struct DeepValue(pub Value);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeepDuration(pub Duration);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeepSystemTime(pub SystemTime);

// --- DeepSizeOf Implementations ---

impl DeepSizeOf for DeepUuid {
    fn deep_size_of_children(&self, _context: &mut deepsize::Context) -> usize {
        // Uuid is a 128-bit value with no heap allocations.
        0
    }
}

impl DeepSizeOf for DeepValue {
    fn deep_size_of_children(&self, _context: &mut deepsize::Context) -> usize {
        // A pragmatic approximation for serde_json::Value.
        self.0.to_string().len()
    }
}

impl DeepSizeOf for DeepDuration {
    fn deep_size_of_children(&self, _context: &mut deepsize::Context) -> usize {
        // Duration is a simple struct with no heap allocations.
        0
    }
}

impl DeepSizeOf for DeepSystemTime {
    fn deep_size_of_children(&self, _context: &mut deepsize::Context) -> usize {
        // SystemTime is a simple struct with no heap allocations.
        0
    }
}

// --- Constructor Implementations ---

impl DeepUuid {
    /// Create a new random UUID
    pub fn new() -> Self {
        DeepUuid(Uuid::new_v4())
    }
}

impl DeepValue {
    /// Create a DeepValue from a string
    pub fn string(s: String) -> Self {
        DeepValue(Value::String(s))
    }
    
    /// Create a DeepValue from an array of values
    pub fn array(arr: Vec<DeepValue>) -> Self {
        let values: Vec<Value> = arr.into_iter().map(|dv| dv.0).collect();
        DeepValue(Value::Array(values))
    }
    
    /// Create a DeepValue from a number
    pub fn number(n: i64) -> Self {
        DeepValue(Value::Number(serde_json::Number::from(n)))
    }
    
    /// Create a DeepValue from a boolean
    pub fn boolean(b: bool) -> Self {
        DeepValue(Value::Bool(b))
    }
    
    /// Create a null DeepValue
    pub fn null() -> Self {
        DeepValue(Value::Null)
    }
}

// --- Display Implementation ---

impl std::fmt::Display for DeepUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Helper Functions for Complex Types ---

pub fn deep_size_of_hashmap_value(
    map: &HashMap<String, DeepValue>,
    context: &mut deepsize::Context,
) -> usize {
    let mut total_size = 0;
    for (key, value) in map.iter() {
        total_size += key.deep_size_of_children(context) + std::mem::size_of_val(key);
        total_size += value.deep_size_of_children(context) + std::mem::size_of_val(value);
    }
    total_size
}

pub fn deep_size_of_option_value(
    opt: &Option<DeepValue>,
    context: &mut deepsize::Context,
) -> usize {
    opt.as_ref()
        .map_or(0, |v| v.deep_size_of_children(context) + std::mem::size_of_val(v))
}

// --- Ergonomic Trait Implementations ---

// Deref and DerefMut to allow calling methods of the inner type directly.
impl Deref for DeepUuid {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for DeepValue {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DeepValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// From implementations for easy conversion.
impl From<Uuid> for DeepUuid {
    fn from(uuid: Uuid) -> Self {
        DeepUuid(uuid)
    }
}

impl From<Value> for DeepValue {
    fn from(value: Value) -> Self {
        DeepValue(value)
    }
}

// Serialize implementations.
impl Serialize for DeepUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl Serialize for DeepValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl Serialize for DeepDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl Serialize for DeepSystemTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        self.0.serialize(serializer)
    }
}


// Deserialize implementations.
impl<'de> Deserialize<'de> for DeepUuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Uuid::deserialize(deserializer).map(DeepUuid)
    }
}

impl<'de> Deserialize<'de> for DeepValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Value::deserialize(deserializer).map(DeepValue)
    }
}

impl<'de> Deserialize<'de> for DeepDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Duration::deserialize(deserializer).map(DeepDuration)
    }
}

impl<'de> Deserialize<'de> for DeepSystemTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        SystemTime::deserialize(deserializer).map(DeepSystemTime)
    }
} 