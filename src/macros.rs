#[allow(unused_imports)]
use crate::StorageData;

/// Macro for generating Storage Types composed of multiple glues ([StorageData]),
/// this macro is used by the derive macro, and it is not meant nor prepared to be
/// manually written by users.
///
/// The format is as follows:
///
/// *vis:vis* *struct:ident* with storage data { <br>
/// &nbsp;&nbsp;       len: *len:literal*, <br>
/// &nbsp;&nbsp;       constructor visibility: *constructor_visibility:vis*, <br>
/// &nbsp;&nbsp;       $({ <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           variable *storage_variable_name:ident*, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           type *storage_type:ty*, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           named *storage_web_name:literal*, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           default {*storage_default:expr*}, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           with storage kind *storage_kind:path*, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           with documentation *storage_doc:literal*, <br>
/// &nbsp;&nbsp;&nbsp;&nbsp;           storage kind for doc *storage_kind_for_doc:literal*, <br>
/// &nbsp;&nbsp;       })*<br>
///    }
///
/// Where every argument means:
///
/// - vis: The visibility of the struct and that of most of the functions.
/// - struct: The name of the struct to generate that will hold all the glues.
/// - len: Amount of glues inside the struct.
/// - constructor_visibility: Visibility of the ***new*** function.
/// - For every glue to create:
///   - storage_variable_name: Name of the variable that will hold the glue.
///   - storage_type: Type of the variable this glue stores.
///   - storage_web_name: Key used in the Web Storage, since it is Web Storage,
/// the conventions don't need to match Rust's.
///   - storage_default: Default value to get when value isn't present in the Storage.
///   - storage_kind: Storage Kind to use, being this either Local or Session.
///   - storage_doc: Documentation of the variable.
///   - storage_kind_for_doc: The name of the Storage Kind used for this Glue, this
/// value is used to tell the name of the storage type in the documentation.
#[macro_export]
macro_rules! define_storage {
    ($vis:vis $struct:ident with storage data {
        len: $len:literal,
        constructor visibility: $constructor_visibility:vis,
        $({
            variable $storage_variable_name:ident,
            type $storage_type:ty,
            named $storage_web_name:literal,
            default {$storage_default:expr},
            $(with storage kind $storage_kind:path,)?
            with documentation $storage_doc:literal,
            storage kind for doc $storage_kind_for_doc:literal,
        })*
    } ) => {
        #[doc = concat!("Glues to Local/Session storages:",
        $( "\n - ", stringify!($storage_variable_name), " in ", $storage_kind_for_doc,
        " Storage: ", $storage_doc, )*
        )]
        $vis struct $struct {
            $(
                #[doc = $storage_doc]
                $vis $storage_variable_name : ::storage_data
                    ::StorageData<&'static str, $storage_type>,
            )*
        }

        impl $struct{
            #[doc = "Creates a new instance where every glue is uninitialized."]
            $constructor_visibility const fn new() -> Self {
                Self {
                    $(
                        $storage_variable_name : ::storage_data::StorageData
                            ::<&'static str, $storage_type>
                            ::new($storage_web_name, || $storage_default)
                            $(.with_storage($storage_kind))?
                            ,
                    )*
                }
            }
            #[doc = "Amount of glues."]
            $vis const fn len(&self) -> usize {
                $len
            }
            #[doc = "Amount of initialized glues."]
            $vis fn len_initialized(&self) -> usize {
                let mut len = 0;
                $(
                    if self.$storage_variable_name.is_set(){
                        len+=1;
                    }
                )*
                len
            }
            #[doc = "Destroys every glue's value, and returns the web names of those that"]
            #[doc = "failed to be deleted."]
            $vis fn clear(&mut self, list_failed_storages:bool) -> Result<(), Vec<&'static str>> {
                let mut failed_storages = Vec::new();
                let mut error = false;
                $(
                    if self.$storage_variable_name.remove().is_err() {
                        error = true;
                        if list_failed_storages{
                            failed_storages.push($storage_web_name);
                        }
                    }
                )*
                if error {
                    Err(failed_storages)
                } else {
                    Ok(())
                }
            }
            #[doc = concat!("Saves the value of every glue, and returns the web names of those \
            that failed.\n\nWhen dropping the glues, all of the glues' values will be  \
            automatically saved if they were modified, meaning this function will be called in \
            your stead once the ", stringify!($struct) ," goes out of scope, except if ",
            stringify!($struct)," is  kept inside a static variable, as these don't automatically \
             call [core::ops::Drop].")]
            $vis fn save(&mut self, list_failed_storages:bool) -> Result<(), Vec<&'static str>> {
                let mut failed_storages = Vec::new();
                let mut error = false;
                $(
                    if self.$storage_variable_name.save().is_err() {
                        error = true;
                        if list_failed_storages{
                            failed_storages.push($storage_web_name);
                        }
                    }
                )*
                if error {
                    Err(failed_storages)
                } else {
                    Ok(())
                }
            }
        }
    };
}