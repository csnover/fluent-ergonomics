//! Provide a more ergonomic interface to the base Fluent library
//!
//! The Fluent class makes it easier to load translation bundles with language fallbacks and to go
//! through the most common steps of translating a message.
//!
use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentError, FluentResource};
use fluent_syntax::parser::ParserError;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::string::FromUtf8Error;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

#[derive(Debug)]
pub enum Error {
    /// All files must be UTF-8 encoded.
    FileEncodingError(FromUtf8Error),
    /// Fluent encountered an underlying error
    FluentError(Vec<FluentError>),
    /// Fluent encountered an underlying error while parsing the translation strings
    FluentParserError(Vec<ParserError>),
    /// There was an underlying IO error
    IOError(io::Error),
    /// No message could be found matching the specified message ID
    NoMatchingMessage(String),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::FileEncodingError(error) => Some(error),
            Error::NoMatchingMessage(_) => None,
            Error::FluentParserError(_) => None,
            Error::FluentError(_) => None,
            Error::IOError(error) => Some(error),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FileEncodingError(error) => {
                write!(f, "Translation file has an encoding problem: {}", error)
            }
            Error::FluentError(errs) => write!(f, "Fluent Error: {:?}", errs),
            Error::FluentParserError(errs) => write!(f, "Fluent Parser Error: {:?}", errs),
            Error::IOError(error) => write!(f, "IO Error: {}", error),
            Error::NoMatchingMessage(id) => write!(f, "No matching message for {}", id),
        }
    }
}

impl From<(FluentResource, Vec<ParserError>)> for Error {
    fn from(inp: (FluentResource, Vec<ParserError>)) -> Self {
        let (_, error) = inp;
        Error::FluentParserError(error)
    }
}

impl From<Vec<ParserError>> for Error {
    fn from(error: Vec<ParserError>) -> Self {
        Error::FluentParserError(error)
    }
}

impl From<Vec<FluentError>> for Error {
    fn from(error: Vec<FluentError>) -> Self {
        Error::FluentError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IOError(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Error::FileEncodingError(error)
    }
}

#[derive(Clone, Default)]
pub struct FluentErgo {
    languages: Vec<LanguageIdentifier>,
    bundles: Arc<RwLock<HashMap<LanguageIdentifier, FluentBundle<FluentResource>>>>,
}

impl fmt::Debug for FluentErgo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FluentErgo")
        //write!(
        //f,
        //"FluentErgo {{ language: {:?}, units: {} }}",
        //self.language, "whatever, for the moment"
        //)
    }
}

/// An Ergonomic class wrapping the Fluent library
impl FluentErgo {
    /// Construct the class with a list of languages. The list must be sorted in the order that
    /// language packs will be tested. The first language listed will be the first language
    /// searched for any translation message.
    ///
    /// Typically, I call this as
    ///
    /// ```
    /// let eo_id = "eo".parse::<unic_langid::LanguageIdentifier>().unwrap();
    /// let en_id = "en-US".parse::<unic_langid::LanguageIdentifier>().unwrap();
    ///
    /// let mut fluent = fluent_ergonomics::FluentErgo::new(&[eo_id, en_id]);
    /// ```
    ///
    /// This specifies that I want to first look up messages in the Esperanto list, then fall back
    /// to the English specfications if no Esperanto specification is present.
    ///
    /// Note that no language resources are loaded during construction. You must call
    /// `add_from_text` or `add_from_file` to load language packs.
    pub fn new(languages: &[LanguageIdentifier]) -> FluentErgo {
        FluentErgo {
            languages: Vec::from(languages),
            bundles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a list of translation strings from a string, which can be a constant hard-coded in the
    /// application, loaded from a file, loaded from the internet, or wherever you like. `lang`
    /// specifies which language the translation strings being provided.
    ///
    /// You should not specify a language that you did not include in the constructor. You can, but
    /// the translation function will only check those languages specified when this object was
    /// constructed.
    ///
    /// # Errors
    ///
    /// * `FluentError`
    /// * `FluentParserError`
    ///
    pub fn add_from_text(&mut self, lang: LanguageIdentifier, text: String) -> Result<(), Error> {
        let res = FluentResource::try_new(text)?;
        let mut bundles = self.bundles.write().unwrap();
        let entry = bundles.entry(lang.clone());
        match entry {
            Entry::Occupied(mut e) => {
                let bundle = e.get_mut();
                bundle.add_resource(res).map_err(|err| Error::from(err))
            }
            Entry::Vacant(e) => {
                let mut bundle = FluentBundle::new(&[lang]);
                bundle.add_resource(res).map_err(|err| Error::from(err))?;
                e.insert(bundle);
                Ok(())
            }
        }?;
        Ok(())
    }

    /// Like `add_from_text`, but this will load the translation strings from a file.
    ///
    /// Note that this will load the entire file into memory before passing it to Fluent. While I
    /// think it is unlikely, it is possible that a translation file may be so big as to run the
    /// computer out of memory.
    ///
    /// # Errors
    ///
    /// * `FluentError`
    /// * `FluentParserError`
    /// * `FileEncodingError` -- all files must be encoded in UTF-8. Most files saved from text
    /// editors already do proper UTF-8 encoding, so this should rarely be a problem.
    ///
    pub fn add_from_file(&mut self, lang: LanguageIdentifier, path: &Path) -> Result<(), Error> {
        let mut v = Vec::new();
        let mut f = File::open(path)?;
        f.read_to_end(&mut v)?;
        String::from_utf8(v)
            .map_err(Error::FileEncodingError)
            .and_then(|s| self.add_from_text(lang, s))
    }

    /// Run a translation.
    ///
    /// `msgid` is the translation identifier as specified in the translation strings. `args` is a
    /// set of Fluent arguments to be interpolated into the strings.
    ///
    /// This function will search language bundles in the order that they were specified in the
    /// constructor. NoMatchingMessage will be returned only if the message identifier cannot be
    /// found in any bundle.
    ///
    /// ```ignore
    /// length-without-label = {$value}
    /// swimming = Swimming
    /// units = Units
    /// ```
    ///
    /// With this set of translation strings, `length-without-label`, `swimming`, and `units` are
    /// all valid translation identifiers. See the documentation for `FluentBundle.get_message` for
    /// more information.
    ///
    /// A typical call with arguments would look like this:
    ///
    /// ```
    /// use fluent::{FluentArgs, FluentValue};
    ///
    /// let eo_id = "eo".parse::<unic_langid::LanguageIdentifier>().unwrap();
    /// let en_id = "en-US".parse::<unic_langid::LanguageIdentifier>().unwrap();
    ///
    /// let mut fluent = fluent_ergonomics::FluentErgo::new(&[eo_id, en_id]);
    /// let mut args = FluentArgs::new();
    /// args.insert("value", FluentValue::from("15"));
    /// let r = fluent.tr("length-without-label", Some(&args));
    /// ```
    ///
    /// # Errors
    ///
    /// * NoMatchingMessage -- this will be returned if the message identifier cannot be found in
    /// any language bundle.
    ///
    pub fn tr(&self, msgid: &str, args: Option<&FluentArgs>) -> Result<String, Error> {
        let bundles = self.bundles.read().unwrap();
        let result: Option<String> = self
            .languages
            .iter()
            .map(|lang| {
                let bundle = bundles.get(lang)?;
                self.tr_(bundle, msgid, args)
            })
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .next();

        match result {
            Some(r) => Ok(r),
            _ => Err(Error::NoMatchingMessage(String::from(msgid))),
        }
    }

    fn tr_(
        &self,
        bundle: &FluentBundle<FluentResource>,
        msgid: &str,
        args: Option<&FluentArgs>,
    ) -> Option<String> {
        let mut errors = vec![];
        let pattern = bundle.get_message(msgid).and_then(|msg| msg.value);
        let res = match pattern {
            None => None,
            Some(p) => {
                let res = bundle.format_pattern(&p, args, &mut errors);
                if errors.len() > 0 {
                    println!("Errors in formatting: {:?}", errors)
                }

                Some(String::from(res))
            }
        };
        match res {
            Some(mut tr_string) => {
                tr_string.retain(|v| v != '\u{2068}' && v != '\u{2069}');
                Some(tr_string)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FluentErgo;
    use fluent::{FluentArgs, FluentValue};
    use unic_langid::LanguageIdentifier;

    const EN_TRANSLATIONS: &'static str = "
preferences = Preferences
history = History
time_display = {$time} during the day
nested_display = nesting a time display: {time_display}
";

    const EO_TRANSLATIONS: &'static str = "
history = Historio
";

    #[test]
    fn translations() {
        let en_id = "en-US".parse::<LanguageIdentifier>().unwrap();
        let mut fluent = FluentErgo::new(&vec![en_id.clone()]);
        fluent
            .add_from_text(en_id, String::from(EN_TRANSLATIONS))
            .expect("text should load");
        assert_eq!(
            fluent.tr("preferences", None).unwrap(),
            String::from("Preferences")
        );
    }

    #[test]
    fn translation_fallback() {
        let eo_id = "eo".parse::<LanguageIdentifier>().unwrap();
        let en_id = "en".parse::<LanguageIdentifier>().unwrap();
        let mut fluent = FluentErgo::new(&vec![eo_id.clone(), en_id.clone()]);
        fluent
            .add_from_text(en_id, String::from(EN_TRANSLATIONS))
            .expect("text should load");
        fluent
            .add_from_text(eo_id, String::from(EO_TRANSLATIONS))
            .expect("text should load");
        assert_eq!(
            fluent.tr("preferences", None).unwrap(),
            String::from("Preferences")
        );
        assert_eq!(
            fluent.tr("history", None).unwrap(),
            String::from("Historio")
        );
    }

    #[test]
    fn placeholder_insertion_should_strip_placeholder_markers() {
        let en_id = "en".parse::<LanguageIdentifier>().unwrap();
        let mut fluent = FluentErgo::new(&vec![en_id.clone()]);
        fluent
            .add_from_text(en_id, String::from(EN_TRANSLATIONS))
            .expect("text should load");
        let mut args = FluentArgs::new();
        args.insert("time", FluentValue::from(String::from("13:00")));
        assert_eq!(
            fluent.tr("time_display", Some(&args)).unwrap(),
            String::from("13:00 during the day")
        );
    }

    #[test]
    fn placeholder_insertion_should_strip_nested_placeholder_markers() {
        let en_id = "en".parse::<LanguageIdentifier>().unwrap();
        let mut fluent = FluentErgo::new(&vec![en_id.clone()]);
        fluent
            .add_from_text(en_id, String::from(EN_TRANSLATIONS))
            .expect("text should load");
        let mut args = FluentArgs::new();
        args.insert("time", FluentValue::from(String::from("13:00")));
        assert_eq!(
            fluent.tr("nested_display", Some(&args)).unwrap(),
            String::from("nesting a time display: 13:00 during the day")
        );
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<FluentErgo>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<FluentErgo>();
    }
}
