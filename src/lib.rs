use fluent::concurrent::FluentBundle;
use fluent::{FluentArgs, FluentError, FluentResource};
use fluent_syntax::parser::ParserError;
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
    NoMatchingMessage,
    FluentParserError(Vec<ParserError>),
    FluentError(Vec<FluentError>),
    IOError(io::Error),
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
    //language: LanguageIdentifier,
    bundle: Arc<RwLock<FluentBundle<FluentResource>>>,
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
    pub fn new(languages: Vec<LanguageIdentifier>) -> FluentErgo {
        let bundle = FluentBundle::new(&languages);
        FluentErgo {
            bundle: Arc::new(RwLock::new(bundle)),
        }
    }

    pub fn add_from_text(&mut self, text: String) -> Result<(), Error> {
        let res = FluentResource::try_new(text)?;
        self.bundle
            .write()
            .unwrap()
            .add_resource(res)
            .map_err(|err| Error::from(err))
    }

    pub fn add_from_file(&mut self, path: &Path) -> Result<(), Error> {
        let mut v = Vec::new();
        let mut f = File::open(path)?;
        f.read_to_end(&mut v)?;
        self.add_from_text(String::from_utf8(v).unwrap())
    }

    /*
    fn add_language(bundle: &mut FluentBundle<FluentResource>, lang: &Language) {
        let lang_resource = match lang {
            Language::English => {
                FluentResource::try_new(load_translation_file(PathBuf::from("en.txt")))
            }
            Language::Esperanto => {
                FluentResource::try_new(load_translation_file(PathBuf::from("eo.txt")))
            }
        };
        match lang_resource {
            Ok(res) => {
                let _ = bundle.add_resource(res);
            }
            Err(err) => panic!("{:?}", err),
        }
    }
    */

    pub fn tr(&self, id: &str, args: Option<&FluentArgs>) -> Result<String, Error> {
        let mut errors = vec![];

        let bundle = self.bundle.read().unwrap();
        let pattern = bundle
            .get_message(id)
            .and_then(|msg| msg.value)
            .ok_or(Error::NoMatchingMessage)?;
        let res = bundle.format_pattern(&pattern, args, &mut errors);
        if errors.len() != 0 {
            Err(Error::from(errors))
        } else {
            Ok(String::from(res))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FluentErgo;
    use unic_langid::LanguageIdentifier;

    const en_translations: &'static str = "
preferences = Preferences
history = History
";

    const eo_translations: &'static str = "
history = Historio
";

    #[test]
    fn translations() {
        //let en = "en-US".parse::<LanguageIdentifier>();
        let mut en = FluentErgo::new(Vec::new());
        en.add_from_text(String::from(en_translations))
            .expect("text should load");
        assert_eq!(
            en.tr("preferences", None).unwrap(),
            String::from("Preferences")
        );
    }

    #[test]
    fn translation_fallback() {
        let eo_id = "eo".parse::<LanguageIdentifier>().unwrap();
        let en_id = "en".parse::<LanguageIdentifier>().unwrap();
        let mut eo = FluentErgo::new(vec![eo_id, en_id]);
        eo.add_from_text(String::from(en_translations))
            .expect("text should load");
        eo.add_from_text(String::from(eo_translations))
            .expect("text should load");
        assert_eq!(
            eo.tr("preferences", None).unwrap(),
            String::from("Preferences")
        );
        assert_eq!(eo.tr("history", None).unwrap(), String::from("Historio"));
    }
}
