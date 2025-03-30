[![crates.io](https://img.shields.io/crates/v/simple_detailed_error.svg)](https://crates.io/crates/simple_detailed_error)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/JorgeRicoVivas/simple_detailed_error/rust.yml)](https://github.com/JorgeRicoVivas/simple_detailed_error/actions)
[![docs.rs](https://img.shields.io/docs.rs/simple_detailed_error)](https://docs.rs/simple_detailed_error/latest/simple_detailed_error/)
[![GitHub License](https://img.shields.io/github/license/JorgeRicoVivas/simple_detailed_error)](https://github.com/JorgeRicoVivas/simple_detailed_error?tab=CC0-1.0-1-ov-file)
![](https://img.shields.io/badge/This%20docs%20version-1.0.0-blue)

This crate allows to easily associate Local/Session storage data through the [StorageData]
struct and to retrieve and set the value without requiring to manually interacting with the Web
Storage API.

For example, if your web app has an item 'MyPreferredColor' whose value is a String, you can
associate it to a [StorageData] like this:

```rust
use storage_data::StorageData;
const DEFAULT_COLOR : &str = "BLUE";

// This creates a StorageData<_, String> containing a String to the preferred color.
// If the value isn't find in the Storage, then DEFAULT_COLOR will be used instead.
let mut preferred_color = StorageData::new("MyPreferredColor", || DEFAULT_COLOR.to_string());

// We can also get the String as a reference
let preferred_color_string : &String = &*preferred_color;
println!("The user preferred color is {}", preferred_color_string);

// The StorageData value can be changed, although it's contents won't be saved in the Browser
// storage immediately, that will happen when the StorageData is dropped or when you manually
// call StorageData::save.
preferred_color.set("GREEN".to_string());

// We can also own the contents of the StorageData, although this destroys the StorageData.
let preferred_color_string : String = preferred_color.take();
```

This eases up manually calling the Web Storage API, but this might still feel uncomfortable to
use, as you need to manually associate every value in a StorageData, to alleviate this, the
[derive_web_storage::WebStorage] derive macro allows you to create a struct where you define a group of storage
values, and it modifies said struct to turn every value into a StorageData.

For example, if you have these values you want to associate to a Storage:

- ``visited_times: usize``: Containing the times the user, visits a page.
- ``picked_products: Vec<String>``: A list of products the user picked for buying, which is a
   value that should be stored in a Session Storage rather than Local.
- ``user_info: UserInfo``: A custom-made struct with personal information about the user.

You could define a storage such as this:

```rust
use derive_web_storage::WebStorage;

#[derive(Debug)]
#[WebStorage(
    // Optional: This prepends every Key of every storage data with 'USER::_::ALT::_'.
    Prepend_keys_with(USER::_::ALT::_),
    // Optional: This changes the default visibility in which this WebStorage can be created.
    ConstructorVisibility(pub(crate)),
    // Optional: This changes the default storage used in every StorageData.
    // Default: The one you set in the features when importing this crate; If you didn't set
    //          a default Storage, LocalStorage will be used.
    StorageKind(Local)
)]
pub struct Storage {
    // It isn't necessary to specify the default value for visited_times as 'usize' implements
    // Default.
    visited_times: usize,

    // This is only saved as a Session value, instead of Local.
    #[StorageKind(Session)]
    picked_products: Vec<String>,

    // This value needs to use a default constructor, as UserInfo doesn't have one.
    // In a real case scenario this should be an Option initialized as None.
    #[default(UserInfo {
        name: "Jorge Rico Vivas".to_string(),
        preferred_color: "Blue".to_string()
    })]
    user_info: UserInfo,
}

struct UserInfo{
    name:String, preferred_color: String,
}

let mut storage = Storage::new();

// Increment visited times.
*storage.visited_times+=1;

// Print info about the user.
println!("The user {} has visited this page {} time(s)",
    storage.user_info.name, storage.visited_times);

// Storage is saved here when dropped, you aren't required to manually save it.
```