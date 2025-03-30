use proc_macro2::{Span, TokenStream};
use proc_macro_error::{Diagnostic, Level};
use syn::Fields;
use syn::spanned::Spanned;

#[must_use]
#[forbid(unused_must_use)]
pub(crate) enum ErrorMessages<'tokens_lf> {
    ExpectedDifferent {
        expected: &'tokens_lf str,
        found: TokenStream,
        span: Span,
    },
    StructFieldsMustBeNamed { fields: Fields },
}

impl<'tokens_lf> ErrorMessages<'tokens_lf> {
    pub(crate) fn abort(self) -> ! {
        self.diagnostic().abort()
    }

    pub(crate) fn diagnostic(self) -> Diagnostic {
        match self {
            ErrorMessages::ExpectedDifferent {
                expected,
                found,
                span,
            } => {
                Diagnostic::spanned(
                    span.into(),
                    Level::Error,
                    format!("Expected {expected}, but found {}.", found.to_string()),
                )
            }
            ErrorMessages::StructFieldsMustBeNamed { fields } => {
                Diagnostic::spanned(
                    fields.span(),
                    Level::Error,
                    "WebStorage macro targets structs with NAMED fields, but this has unnamed fields.".to_string(),
                )
            }
        }
    }
}
