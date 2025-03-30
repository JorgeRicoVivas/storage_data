use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use core::error::Error;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, variadic)]
    pub fn error(items: Box<[JsValue]>);
}

#[inline]
pub fn log_error(message: &str) {
    if cfg!(debug_assertions) {
        let loc = core::panic::Location::caller();
        let msg = format!(
            "{} ({}:{}:{})",
            message,
            loc.file(),
            loc.line(),
            loc.column()
        );
        error(Box::from([JsValue::from(&msg)]));
    } else {
        error(Box::from([JsValue::from(message)]));
    }
}

pub trait LogError {
    type SucessType;
    type ErrorType;
    fn log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Self
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description;

    fn map_log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Result<Self::SucessType, Box<dyn Error>>
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description;
}

impl<T> LogError for Option<T> {
    type SucessType = T;
    type ErrorType = ();

    fn log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Self
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description,
    {
        match &self {
            None => log_error(error_descriptor(&()).as_ref()),
            Some(_) => {}
        };
        self
    }

    fn map_log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Result<Self::SucessType, Box<dyn Error>>
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description,
    {
        match self {
            None => {
                let error_descriptor = error_descriptor(&()).as_ref().to_string();
                log_error(error_descriptor.as_ref());
                Err(error_descriptor.into())
            }
            Some(v) => Ok(v),
        }
    }
}

impl<T, E> LogError for Result<T, E> {
    type SucessType = T;
    type ErrorType = E;

    fn log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Self
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description,
    {
        match &self {
            Err(err) => log_error(&error_descriptor(err).as_ref()),
            Ok(_) => {}
        };
        self
    }

    fn map_log_possible_error<Description, DescriptionGetter>(
        self,
        error_descriptor: DescriptionGetter,
    ) -> Result<Self::SucessType, Box<dyn Error>>
    where
        Description: AsRef<str>,
        DescriptionGetter: FnOnce(&Self::ErrorType) -> Description,
    {
        match self {
            Err(err) => {
                let error = error_descriptor(&err).as_ref().to_string();
                log_error(&*error);
                Err(error.into())
            }
            Ok(v) => Ok(v),
        }
    }
}
