use mruby::interpreter::Mrb;
use mruby::load::MrbLoadSources;
use mruby::MrbError;
use std::borrow::Cow;
use std::convert::AsRef;

use crate::Gem;

/// Rack gem at version 2.0.7.
#[derive(RustEmbed)]
// TODO: resolve path relative to CARGO_MANIFEST_DIR
// https://github.com/pyros2097/rust-embed/pull/59
#[folder = "mruby-gems/vendor/ruby/2.6.0/gems/rack-2.0.7/lib"]
pub struct Rack;

impl Rack {
    fn contents<T: AsRef<str>>(path: T) -> Result<Vec<u8>, MrbError> {
        let path = path.as_ref();
        Self::get(path)
            .map(Cow::into_owned)
            .ok_or_else(|| MrbError::SourceNotFound(path.to_owned()))
    }
}

impl Gem for Rack {
    fn install(interp: &mut Mrb) -> Result<(), MrbError> {
        for source in Self::iter() {
            let contents = Self::contents(&source)?;
            interp.def_rb_source_file(source, contents)?;
        }
        Ok(())
    }
}
