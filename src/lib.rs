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
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

#[derive(Debug)]
pub enum Error {
    NoMatchingMessage(String),
    FluentParserError(Vec<ParserError>),
    FluentError(Vec<FluentError>),
    IOError(io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
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
            Error::NoMatchingMessage(id) => write!(f, "No matching message for {}", id),
            Error::FluentParserError(errs) => write!(f, "Fluent Parser Error: {:?}", errs),
            Error::FluentError(errs) => write!(f, "Fluent Error: {:?}", errs),
            Error::IOError(error) => write!(f, "IO Error: {}", error),
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

#[derive(Clone)]
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

impl FluentErgo {
    pub fn new(languages: &[LanguageIdentifier]) -> FluentErgo {
        FluentErgo {
            languages: Vec::from(languages),
            bundles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

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

    pub fn add_from_file(&mut self, lang: LanguageIdentifier, path: &Path) -> Result<(), Error> {
        let mut v = Vec::new();
        let mut f = File::open(path)?;
        f.read_to_end(&mut v)?;
        self.add_from_text(lang, String::from_utf8(v).unwrap())
    }

    pub fn tr(&self, msgid: &str, args: Option<&FluentArgs>) -> Result<String, Error> {
        let bundles = self.bundles.read().unwrap();
        let result: Option<String> = self
            .languages
            .iter()
            .map(|lang| {
                println!("trying language: {:?}", lang);
                let bundle = bundles.get(lang)?;
                self.tr_(bundle, msgid, args)
            })
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .next();

        println!("result: {:?}", result);
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
        match pattern {
            None => None,
            Some(p) => {
                let res = bundle.format_pattern(&p, args, &mut errors);
                if errors.len() > 0 {
                    println!("Errors in formatting: {:?}", errors)
                }

                Some(String::from(res))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FluentErgo;
    use unic_langid::LanguageIdentifier;

    const EN_TRANSLATIONS: &'static str = "
preferences = Preferences
history = History
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
}
