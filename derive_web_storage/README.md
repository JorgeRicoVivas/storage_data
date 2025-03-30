Refer to the [storage_data](https://crates.io/crates/storage_data) crate, don't use this crate
independently of it.

The ``storage_data`` crate allows to easily associate Local/Session storage data through the
StorageData struct and to retrieve and set the value without requiring to manually interacting
with the Web Storage API, and this crate is made to allow creating a struct where associating
multiple StorageData is made much simpler and maintainable.