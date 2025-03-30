use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::error::Error;
#[cfg(any(feature = "use_serde_bincode", feature = "use_serde_cbor"))]
use base64::Engine;
use crate::log_error::LogError;

#[cfg(any(feature = "use_serde_bincode", feature = "use_serde_cbor"))]
const GENERAL_PURPOSE_ENCODER: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE;

#[cfg(feature = "use_serde_json")]
pub(crate) fn serialize_json<Value: serde::Serialize>(value: &Value)
    -> Result<String, Box<dyn Error>> {
    serde_json::to_string(&value)
        .map_log_possible_error(|err| format!("Cannot serialize as json due to {err:?}"))
}

#[cfg(feature = "use_serde_json")]
pub(crate) fn deserialize_json<Value: for<'de> serde::de::Deserialize<'de>>(serialized: String)
    -> Result<Value, Box<dyn Error>> {
    serde_json::from_str(&serialized)
        .map_log_possible_error(|err| format!("Cannot deserialize as json due to {err:?}"))
}

#[cfg(feature = "use_serde_bincode")]
pub(crate) fn serialize_bincode<Value: serde::Serialize>(value: &Value)
    -> Result<String, Box<dyn Error>> {
    let deserialized = bincode::serialize(&value)
        .map_log_possible_error(|err| format!("Cannot serialize as bincode due to {err:?}"))?;
    Ok(GENERAL_PURPOSE_ENCODER.encode(deserialized))
}

#[cfg(feature = "use_serde_bincode")]
pub(crate) fn deserialize_bincode<Value: for<'de> serde::de::Deserialize<'de>>(serialized: String)
    -> Result<Value, Box<dyn Error>> {
    let serialized = GENERAL_PURPOSE_ENCODER.decode(serialized.as_bytes())
        .map_log_possible_error(|err|
            format!("Cannot decode on deserialization of bincode due to {err:?}"))?;
    bincode::deserialize(&*serialized)
        .map_log_possible_error(|err| format!("Cannot deserialize as bincode due to {err:?}"))
}

#[cfg(feature = "use_serde_yaml")]
pub(crate) fn serialize_yaml<Value: serde::Serialize>(value: &Value)
    -> Result<String, Box<dyn Error>> {
    serde_yaml::to_string(&value)
        .map_log_possible_error(|err| format!("Cannot serialize as yaml due to {err:?}"))
}

#[cfg(feature = "use_serde_yaml")]
pub(crate) fn deserialize_yaml<Value: for<'de> serde::de::Deserialize<'de>>(serialized: String)
    -> Result<Value, Box<dyn Error>> {
    serde_yaml::from_str(&serialized)
        .map_log_possible_error(|err| format!("Cannot deserialize as yaml due to {err:?}"))
}

#[cfg(feature = "use_serde_ron")]
pub(crate) fn serialize_ron<Value: serde::Serialize>(value: &Value)
    -> Result<String, Box<dyn Error>> {
    ron::to_string(&value)
        .map_log_possible_error(|err| format!("Cannot serialize as RON due to {err:?}"))
}

#[cfg(feature = "use_serde_ron")]
pub(crate) fn deserialize_ron<Value: for<'de> serde::de::Deserialize<'de>>(serialized: String)
    -> Result<Value, Box<dyn Error>> {
    ron::from_str(&serialized)
        .map_log_possible_error(|err| format!("Cannot deserialize as RON due to {err:?}"))
}

#[cfg(feature = "use_serde_cbor")]
pub(crate) fn serialize_cbor<Value: serde::Serialize>(value: &Value)
    -> Result<String, Box<dyn Error>> {
    let mut deserialized = alloc::vec::Vec::new();
    ciborium::ser::into_writer(&value, &mut deserialized)
        .map_log_possible_error(|err| format!("Could deserialize as CBOR due to: {err:?}"))?;
    Ok(GENERAL_PURPOSE_ENCODER.encode(deserialized))
}

#[cfg(feature = "use_serde_cbor")]
pub(crate) fn deserialize_cbor<Value: for<'de> serde::de::Deserialize<'de>>(serialized: String)
    -> Result<Value, Box<dyn Error>> {
    let serialized = GENERAL_PURPOSE_ENCODER.decode(serialized.as_bytes())
        .map_log_possible_error(|err|
            format!("Cannot decode on deserialization of bincode due to {err:?}"))?;
    ciborium::de::from_reader(&*serialized)
        .map_log_possible_error(|err| format!("Cannot deserialize as CBOR due to {err:?}"))
}