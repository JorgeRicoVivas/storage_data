//! [![crates.io](https://img.shields.io/crates/v/simple_detailed_error.svg)](https://crates.io/crates/simple_detailed_error)
//! [![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/JorgeRicoVivas/simple_detailed_error/rust.yml)](https://github.com/JorgeRicoVivas/simple_detailed_error/actions)
//! [![docs.rs](https://img.shields.io/docs.rs/simple_detailed_error)](https://docs.rs/simple_detailed_error/latest/simple_detailed_error/)
//! [![GitHub License](https://img.shields.io/github/license/JorgeRicoVivas/simple_detailed_error)](https://github.com/JorgeRicoVivas/simple_detailed_error?tab=CC0-1.0-1-ov-file)
//!
//! This crate allows to easily associate Local/Session storage data through the [StorageData]
//! struct and to retrieve and set the value without requiring to manually interacting with the Web
//! Storage API.
//!
//! For example, if your web app has an item 'MyPreferredColor' whose value is a String, you can
//! associate it to a [StorageData] like this:
//!
//! ```rust
//! use storage_data::StorageData;
//! const DEFAULT_COLOR : &str = "BLUE";
//!
//! // This creates a StorageData<_, String> containing a String to the preferred color.
//! // If the value isn't find in the Storage, then DEFAULT_COLOR will be used instead.
//! let mut preferred_color = StorageData::new("MyPreferredColor", || DEFAULT_COLOR.to_string());
//!
//! // We can also get the String as a reference
//! let preferred_color_string : &String = &*preferred_color;
//! println!("The user preferred color is {}", preferred_color_string);
//!
//! // The StorageData value can be changed, although it's contents won't be saved in the Browser
//! // storage immediately, that will happen when the StorageData is dropped or when you manually
//! // call StorageData::save.
//! preferred_color.set("GREEN".to_string());
//!
//! // We can also own the contents of the StorageData, although this destroys the StorageData.
//! let preferred_color_string : String = preferred_color.take();
//! ```
//!
//! This eases up manually calling the Web Storage API, but this might still feel uncomfortable to
//! use, as you need to manually associate every value in a StorageData, to alleviate this, the
//! [derive_web_storage::WebStorage] derive macro allows you to create a struct where you define a group of storage
//! values, and it modifies said struct to turn every value into a StorageData.
//!
//! For example, if you have these values you want to associate to a Storage:
//!
//! - ``visited_times: usize``: Containing the times the user, visits a page.
//! - ``picked_products: Vec<String>``: A list of products the user picked for buying, which is a
//!    value that should be stored in a Session Storage rather than Local.
//! - ``user_info: UserInfo``: A custom-made struct with personal information about the user.
//!
//! You could define a storage such as this:
//!
//! ```rust
//! use derive_web_storage::WebStorage;
//!
//! #[derive(Debug)]
//! #[WebStorage(
//!     // Optional: This prepends every Key of every storage data with 'USER::_::ALT::_'.
//!     Prepend_keys_with(USER::_::ALT::_),
//!     // Optional: This changes the default visibility in which this WebStorage can be created.
//!     ConstructorVisibility(pub(crate)),
//!     // Optional: This changes the default storage used in every StorageData.
//!     // Default: The one you set in the features when importing this crate; If you didn't set
//!     //          a default Storage, LocalStorage will be used.
//!     StorageKind(Local)
//! )]
//! pub struct Storage {
//!     // It isn't necessary to specify the default value for visited_times as 'usize' implements
//!     // Default.
//!     visited_times: usize,
//!
//!     // This is only saved as a Session value, instead of Local.
//!     #[StorageKind(Session)]
//!     picked_products: Vec<String>,
//!
//!     // This value needs to use a default constructor, as UserInfo doesn't have one.
//!     // In a real case scenario this should be an Option initialized as None.
//!     #[default(UserInfo {
//!         name: "Jorge Rico Vivas".to_string(),
//!         preferred_color: "Blue".to_string()
//!     })]
//!     user_info: UserInfo,
//! }
//!
//! struct UserInfo{
//!     name:String, preferred_color: String,
//! }
//!
//! let mut storage = Storage::new();
//!
//! // Increment visited times.
//! *storage.visited_times+=1;
//!
//! // Print info about the user.
//! println!("The user {} has visited this page {} time(s)",
//!     storage.user_info.name, storage.visited_times);
//!
//! // Storage is saved here when dropped, you aren't required to manually save it.
//! ```

#![no_std]
extern crate alloc;
#[cfg(feature = "derive")]
pub extern crate derive_web_storage;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::convert::AsRef;
use core::error::Error;
use core::fmt::{Debug, Display, Formatter};
use core::ops::{Deref, DerefMut};
use log_error::LogError;
use once_cell::sync::OnceCell;
use web_sys::wasm_bindgen::__rt::core;
pub(crate) mod log_error;
pub(crate) mod serdes;

pub(crate) mod macros;

//todo!(Allow to panic when data couldn't deserialize due to corruption)

//todo!(Macro should also allow to deserialize in a specific way)


/// Web Storage is made of two kinds of storages:
///
/// - Local: Persistent data, use this if you want data to be kept even if the user closes its
/// web browser.
/// - Session: Only kept as long as your web page is open in the user's web browser, if he closes
/// either the tab or the web browser, it will be removed.
///
/// For more information visit: <https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API>.
pub enum StorageKind {
    /// In this Storage the data is persistent.
    Local,
    /// In this Storage the data is kept as long as the web page is open in the web browser.
    Session,
}
impl StorageKind {
    /// Returns the [web_sys::Storage] corresponding to this storage kind.
    pub fn web_sys_storage(&self) -> Result<web_sys::Storage, Box<dyn Error>> {
        let window = web_sys::window().map_log_possible_error(|_| "Could not get windows")?;
        match self {
            StorageKind::Local => window
                .local_storage()
                .map_log_possible_error(|err| format!("Could not get Local Storage ({err:?})"))?
                .map_log_possible_error(|_| "Could not get Local Storage"),
            StorageKind::Session => window
                .session_storage()
                .map_log_possible_error(|err| format!("Could not get Session Storage ({err:?})"))?
                .map_log_possible_error(|_| "Could not get Session Storage"),
        }
    }
    /// Gets an item using this item's key.
    pub fn get_item(&self, key: &str) -> Result<Option<String>, Box<dyn Error>> {
        self.web_sys_storage()?
            .get_item(key)
            .map_log_possible_error(|_| format!("Could not get serialized value for key {key}"))
    }
    /// Sets the value of an item using this item's key.
    pub fn set_item<SerializedValue>(
        &self,
        key: &str,
        value: SerializedValue,
    ) -> Result<(), Box<dyn Error>>
    where
        SerializedValue: FnOnce() -> Result<String, Box<dyn Error>>,
    {
        self.web_sys_storage()?
            .set_item(key, &value()?)
            .map_log_possible_error(|err| {
                format!("Could set serialized value for key {key} due to {err:?}")
            })?;
        Ok(())
    }
    /// Removes the key and value of an item.
    pub fn remove_item(&self, key: &str) -> Result<(), Box<dyn Error>> {
        self.web_sys_storage()?
            .remove_item(key)
            .map_log_possible_error(|err| format!("Could remove value of key {key} due to {err:?}"))
    }
}
/// Gets the value contained in the specified key for this storage kind, and if the
/// storage doesn't contain said key, it returns the default value indicated by parameter.
///
/// This operation can fail if the item could not be deserialized, as Storage only store [String]s
/// on which we can represent this value as a serialized value, specifying this deserialization
/// error as an ``Err<Box<dyn<Error>>>``.
pub fn get_data_with<Key, Value, DefaultValue, Deserialize>(
    storage_kind: &StorageKind,
    key: Key,
    default: DefaultValue,
    deserialize: Deserialize,
) -> Result<Value, Box<dyn Error>>
where
    Key: AsRef<str>,
    DefaultValue: FnOnce() -> Value,
    Deserialize: FnOnce(String) -> Result<Value, Box<dyn Error>>,
{
    let key = key.as_ref();
    match storage_kind.get_item(key).ok().flatten().map(|as_string| {
        deserialize(as_string).map_log_possible_error(|error| {
            format!("Could not deserialize item for key {key} due to:\n{error}")
        })
    }) {
        None => Ok(default()),
        Some(Ok(value)) => Ok(value),
        Some(Err(err)) => Err(err),
    }
}
/// Sets the specified value as serialized string over the specified key for this storage kind.
///
/// This operation can fail if the item could not be serialized, specifying this error
/// as an ``Err<Box<dyn<Error>>>``.
pub fn set_data<Key, Value, Serialize>(
    storage_kind: &StorageKind,
    key: Key,
    value: &Value,
    serialize: Serialize,
) -> Result<(), Box<dyn Error>>
where
    Key: AsRef<str>,
    Serialize: FnOnce(&Value) -> Result<String, Box<dyn Error>>,
{
    let key = key.as_ref();
    storage_kind.set_item(key, || {
        serialize(&value).log_possible_error(|error| {
            format!("Could not serialize item for key {key} due to:\n{error:?}")
        })
    })?;
    Ok(())
}
/// Glue over a Local/Session Storage key and its value.
///
/// Used to retrieve and set the value without requiring to manually interacting with the Web
/// Storage API.
pub struct StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    storage_kind: StorageKind,
    key: Key,
    value: OnceCell<Value>,
    default_value: fn() -> Value,
    panic_on_cannot_deserialize: bool,
    save_on_drop: bool,
    deserialize_as: fn(String) -> Result<Value, Box<dyn Error>>,
    serialize_as: fn(&Value) -> Result<String, Box<dyn Error>>,
    mutated: bool,
}
#[cfg(feature = "default_storage_local")]
/// Default storage used for new [StorageData]s, it is currently set to Local Storage.
#[cfg(feature = "default_storage_local")]
pub const DEFAULT_STORAGE_KIND: StorageKind = StorageKind::Local;
#[cfg(feature = "default_storage_session")]
/// Default storage used for new [StorageData]s, it is currently set to Session Storage.
#[cfg(feature = "default_storage_session")]
pub const DEFAULT_STORAGE_KIND: StorageKind = StorageKind::Session;


impl<Key, Value> StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    /// Creates a glue to the key indicated.
    ///
    /// When trying to retrieve the value, this might not be set yet in the storage,
    /// in which case the default value is got back from the indicated closure.
    pub const fn new(key: Key, default: fn() -> Value) -> Self {
        Self {
            storage_kind: DEFAULT_STORAGE_KIND,

            key,
            value: OnceCell::new(),
            default_value: default,
            panic_on_cannot_deserialize: true,
            save_on_drop: true,
            mutated: false,

            #[cfg(feature = "default_serde_json")]
            serialize_as: serdes::serialize_json,
            #[cfg(feature = "default_serde_json")]
            deserialize_as: serdes::deserialize_json,

            #[cfg(feature = "default_serde_bincode")]
            serialize_as: serdes::serialize_bincode,
            #[cfg(feature = "default_serde_bincode")]
            deserialize_as: serdes::deserialize_bincode,

            #[cfg(feature = "default_serde_yaml")]
            serialize_as: serdes::serialize_yaml,
            #[cfg(feature = "default_serde_yaml")]
            deserialize_as: serdes::deserialize_yaml,

            #[cfg(feature = "default_serde_ron")]
            serialize_as: serdes::serialize_ron,
            #[cfg(feature = "default_serde_ron")]
            deserialize_as: serdes::deserialize_ron,

            #[cfg(feature = "default_serde_cbor")]
            serialize_as: serdes::serialize_cbor,
            #[cfg(feature = "default_serde_cbor")]
            deserialize_as: serdes::deserialize_cbor,
        }
    }
}
impl<Key, Value> StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    /// Specifies whether this value is automatically saved when the [StorageData] is drop.
    ///
    /// Note: Updates only happen when the value has been updated or if the key isn't present in the
    /// storage, meaning only the truly necessary saves are executed.
    pub const fn save_on_drop(mut self, update_on_drop: bool) -> Self {
        self.save_on_drop = update_on_drop;
        self
    }

    /// Specifies the kind of storage this glue targets to, being this either Local or Session.
    pub const fn with_storage(mut self, storage_kind: StorageKind) -> Self {
        self.storage_kind = storage_kind;
        self
    }

    /// Specifies this glue targets Local Storage.
    pub const fn with_local_storage(self) -> Self {
        self.with_storage(StorageKind::Local)
    }

    /// Specifies this glue targets Session Storage.
    pub const fn with_session_storage(self) -> Self {
        self.with_storage(StorageKind::Session)
    }

    /// Specifies how the value is serialized when setting on the Storage.
    pub const fn serialize_with(
        mut self,
        serialize: fn(&Value) -> Result<String, Box<dyn Error>>,
    ) -> Self {
        self.serialize_as = serialize;
        self
    }

    /// Specifies how the value is deserialized when retrieving it from the Storage.
    pub const fn deserialize_with(
        mut self,
        serialize: fn(String) -> Result<Value, Box<dyn Error>>,
    ) -> Self {
        self.deserialize_as = serialize;
        self
    }

    /// Specifies how the value is serialized when setting on the Storage and how it is deserialized
    /// when retrieving it from the Storage.
    pub const fn serde_with(
        self,
        serialize: fn(&Value) -> Result<String, Box<dyn Error>>,
        deserialize: fn(String) -> Result<Value, Box<dyn Error>>,
    ) -> Self {
        self.serialize_with(serialize).deserialize_with(deserialize)
    }

    /// Sets serialization and deserialization as JSON's.
    #[cfg(feature = "use_serde_json")]
    pub const fn serde_json(self) -> Self {
        self.serde_with(serdes::serialize_json, serdes::deserialize_json)
    }

    /// Sets serialization and deserialization as bincode's.
    #[cfg(feature = "use_serde_bincode")]
    pub const fn serde_bincode(self) -> Self {
        self.serde_with(serdes::serialize_bincode, serdes::deserialize_bincode)
    }

    /// Sets serialization and deserialization as YAML's.
    #[cfg(feature = "use_serde_yaml")]
    pub const fn serde_yaml(self) -> Self {
        self.serde_with(serdes::serialize_yaml, serdes::deserialize_yaml)
    }

    /// Sets serialization and deserialization as RON's.
    #[cfg(feature = "use_serde_ron")]
    pub const fn serde_ron(self) -> Self {
        self.serde_with(serdes::serialize_ron, serdes::deserialize_ron)
    }

    /// Sets serialization and deserialization as cbor's.
    #[cfg(feature = "use_serde_cbor")]
    pub const fn serde_cbor(self) -> Self {
        self.serde_with(serdes::serialize_cbor, serdes::deserialize_cbor)
    }

    /// Gets the current value, if is not set, it retrieves it from the Storage through a
    /// deserialization, and if not present, it gets it as the default value.
    fn resolve(&self) -> &Value {
        self.value.get_or_init(|| {
            let value = get_data_with(
                &self.storage_kind,
                self.key.as_ref(),
                self.default_value,
                self.deserialize_as,
            );
            match (value, self.panic_on_cannot_deserialize) {
                (Ok(value), _) => value,
                (Err(_), false) => (self.default_value)(),
                (Err(error), true) => panic!("{error}"),
            }
        })
    }

    /// Gets the current value from the glue.
    ///
    /// This might not be up to date with the Storage if the Storage is modified outside this
    /// [StorageData].
    pub fn get(&self) -> &Value {
        self.resolve()
    }

    /// Gets the current value from the glue.
    ///
    /// This might not be up to date with the Storage if the Storage is modified outside this
    /// [StorageData].
    pub fn get_mut(&mut self) -> &mut Value {
        self.resolve();
        self.value.get_mut().unwrap()
    }

    /// Sets the value for both this glue and the Storage, meaning the Storage should be up to date
    /// after calling this function.
    ///
    /// Saving the result in the Storage might fail, for example, if the value could not be
    /// serialized, or if the quota's limit is reached, returning an explanation to this through an
    /// ``Err<Box<dyn Error>>``.
    pub fn set(&mut self, value: Value) -> Result<(), Box<dyn Error>> {
        let res = set_data(&self.storage_kind, &self.key, &value, self.serialize_as);
        let couldnt_set_and_it_was_initialized =
            self.value.set(value).is_err() && self.value.get().is_some();
        if couldnt_set_and_it_was_initialized {
            self.value = OnceCell::new();
        }
        res
    }

    /// Tells whether this glue holds a value or the Storage has the key.
    pub fn is_set(&self) -> bool {
        self.value.get().is_some() || self.storage_kind.get_item(self.key.as_ref()).is_ok()
    }

    /// Removes both the value of the glue and the value in the Storage.
    ///
    /// This might fail for a variety of reasons, returning an explanation through
    /// an ``Err<Box<dyn Error>>``.
    pub fn remove(&mut self) -> Result<(), Box<dyn Error>> {
        self.storage_kind.remove_item(self.key.as_ref())?;
        self.finalize_use(true, false);
        Ok(())
    }

    /// Takes the value of the glue as an owned value, and if not set, it gets it
    /// from the Storage, and in case the key isn't present in the Storage, it returns
    /// the default value.
    pub fn take(mut self) -> Value {
        self.finalize_use(false, true);
        self.resolve();
        self.value.take().unwrap_or_else(self.default_value)
    }

    /// Sets the current value in the glue over the Storage, meaning the Storage
    /// should be up to date after calling this function.
    ///
    /// Saving the result in the Storage might fail, for example, if the value
    /// could not be serialized, or if the quota's limit is reached, returning
    /// an explanation to this through an ``Err<Box<dyn Error>>``.
    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let was_changed = self.mutated && self.value.get().is_some();
        let storage_contains_this_key = self.storage_kind.get_item(self.key.as_ref()).is_ok();
        if !was_changed && storage_contains_this_key {
            return Ok(());
        }
        let res = set_data(
            &self.storage_kind,
            &self.key,
            self.resolve(),
            self.serialize_as,
        );
        if res.is_ok() {
            self.mutated = false;
        };
        res
    }
    /// Finalization means updating the value if necessary and queried, and clear it if queried.
    fn finalize_use(&mut self, clear: bool, save: bool) {
        if save && self.save_on_drop {
            let _ = self.save();
        }
        if clear {
            self.value = OnceCell::new();
        }
        self.mutated = false;
    }
}

/// Dereferences to the Storage's current glue data through a call to [StorageData::get].
impl<Key, Value> Deref for StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    type Target = Value;
    /// Dereferences to the Storage's current glue data through a call to [StorageData::get].
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

/// Dereferences to the Storage's current glue data through a call to [StorageData::get_mut].
///
/// Calling this means the value will probably mutate, so the value gets marked as mutated
/// once this is called, even if the value doesn't mutate in the end.
impl<Key, Value> DerefMut for StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    /// Dereferences to the Storage's current glue data through a call to [StorageData::get_mut].
    ///
    /// Calling this means the value will probably mutate, so the value gets marked as mutated
    /// once this is called, even if the value doesn't mutate in the end.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        self.get_mut()
    }
}

/// Upon drop, this value is tried to be saved, only if [StorageData::save_on_drop]
/// isn't manually set as false.
impl<Key, Value> Drop for StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
{
    /// Upon drop, this value is tried to be saved, only if [StorageData::save_on_drop]
    /// isn't manually set as false.
    fn drop(&mut self) {
        self.finalize_use(true, true);
    }
}

/// Displays this glue value by getting it through [StorageData::get].
impl<Key, Value> Display for StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de> + Display,
{
    /// Displays this glue value by getting it through [StorageData::get].
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("{}", self.get()))
    }
}

/// Formats as debug using this glue value by getting it through [StorageData::get].
impl<Key, Value> Debug for StorageData<Key, Value>
where
    Key: AsRef<str>,
    Value: serde::Serialize + for<'de> serde::de::Deserialize<'de> + Debug,
{
    /// Formats as debug using this glue value by getting it through [StorageData::get].
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("{:?}", self.get()))
    }
}
