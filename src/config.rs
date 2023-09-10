/// A trait to be implemented by configuration structs. Any assignable fields
/// must be of an optional type, like for example, `Option<bool>`, or
/// `Option<PathBuf>`.
pub trait Configuration: Default {
    /// A method that should return a new configuration `struct` with all
    /// assignable fields set to `None`. Any assignable fields not set to None
    /// should not be loaded/replaced by other implemented methods of this
    /// trait.
    fn new() -> Self;

    /// Replace any unassigned fields (which have the value `None`) from the
    /// supplied config struct if that struct has the relevant fields set.
    /// This method must call `self.set_loaded()` if any fields were
    /// set/modified.
    fn config(&mut self, other: Self) -> &mut Self;

    /// Like [`config()`], but the supplied config struct is a variant of
    /// `Option`
    fn optional_config(&mut self, optional_other: Option<Self>) -> &mut Self {
        if let Some(other) = optional_other {
            self.config(other);
        }
        self
    }

    /// Replace any unassigned fields (which have the value `None`) from the
    /// supplied environmental variables if the relevant environmental
    /// variables are set. This method must call `self.set_loaded()` if any
    /// fields were set/modified.
    fn env(&mut self) -> &mut Self;

    #[cfg(feature = "serde")]
    /// Replace any unassigned fields (which have the value `None`) from a
    /// config string if the string is valid and has the relevant fields set.
    /// If the config string has an invalid format, an error is returned. This
    /// method must call `self.set_loaded()` if any fields were set/modified.  
    /// The meanings of the named lifetimes, associate types, and generic types
    /// are as follows:
    /// - [`S`]: The string-like object which contains the configuration in one
    /// of the supported formats
    /// - [`D`]: A format selector generic type which is a serde `Deserializer`
    /// trait implementor and also implements `From<S>` to permit being created
    /// from a string. In practice, one can create a newtype to wrap the
    /// Deserializer struct (not trait this time) which can be converted to the
    /// configuration struct using standard serde methods. This crate
    /// implements sample format selectors for some common types like YAML
    /// ([`YamlFormat`]), but one can implement custom format selectors
    /// anywhere in a similar manner.
    fn string<'de, D>(&mut self, config_string: &'de str) -> Result<&mut Self, Error>
    where
        Self: Deserialize<'de> + 'de,
        D: ConfigDeserialize<'de, Self, Error = Box<dyn std::error::Error + 'static>>,
    {
        let other_config = D::try_config_from_string(config_string)
            .map_err(|serde_error| Box::from(serde_error))
            .context(ParseConfigStringSnafu {
                string: config_string.clone(),
            })?;
        self.config(other_config);
        self.set_loaded();
        Ok(self)
    }

    #[cfg(feature = "serde")]
    /// Replace any unassigned fields (which have the value `None`) from a
    /// config filepath if the file at the supplied filepath is valid and has
    /// the relevant fields set. If a file does not exist at that filepath,
    /// the method fails silently. However, if the file exists but cannot be
    /// read, or if the file has an invalid format, an error is returned. This
    /// method must call `self.set_loaded()` if any fields were set/modified.  
    fn filepath<'de, D>(&mut self, config_filepath: impl AsRef<Path>) -> Result<&mut Self, Error>
    where
        Self: Deserialize<'de> + 'de,
        D: ConfigDeserialize<'de, Self, Error = Box<dyn std::error::Error + 'static>>,
    {
        let config_filepath = config_filepath.as_ref().to_owned();
        if !config_filepath.exists() {
            Ok(self)
        } else {
            let file = File::open(config_filepath.clone()).context(ReadConfigFileSnafu {
                path: config_filepath.clone(),
            })?;
            let file_reader = BufReader::new(file);
            let other_config = D::try_config_from_reader(file_reader)
                .map_err(|serde_error| Box::from(serde_error))
                .context(ParseConfigFileSnafu {
                    path: config_filepath.clone(),
                })?;
            self.config(other_config);
            self.set_loaded();
            Ok(self)
        }
    }

    #[cfg(feature = "serde")]
    /// Like [`filepath()`], but takes an optional filepath
    fn optional_filepath<'de, D>(
        &mut self,
        optional_config_filepath: Option<impl AsRef<Path>>,
    ) -> Result<&mut Self, Error>
    where
        Self: Deserialize<'de> + 'de,
        D: ConfigDeserialize<'de, Self, Error = Box<dyn std::error::Error + 'static>>,
    {
        match optional_config_filepath {
            Some(config_filepath) => self.filepath::<D>(config_filepath),
            None => Ok(self),
        }
    }

    #[cfg(feature = "serde")]
    /// Like [`filepath()`], but additionally also fails when a file does not
    /// exist at the given config_filepath.
    fn try_filepath<'de, D>(
        &mut self,
        config_filepath: impl AsRef<Path>,
    ) -> Result<&mut Self, Error>
    where
        Self: Deserialize<'de> + 'de,
        D: ConfigDeserialize<'de, Self, Error = Box<dyn std::error::Error + 'static>>,
    {
        if !config_filepath.exists() {
            Err(Error::FindConfigFile {
                path: config_filepath.as_ref().to_owned(),
            })
        } else {
            self.filepath::<D>(config_filepath)
        }
    }

    #[cfg(feature = "serde")]
    /// Like [`try_filepath()`], but takes an optional filepath.
    fn try_optional_filepath<'de, D>(
        &mut self,
        optional_config_filepath: Option<impl AsRef<Path>>,
    ) -> Result<&mut Self, Error>
    where
        Self: Deserialize<'de> + 'de,
        D: ConfigDeserialize<'de, Self, Error = Box<dyn std::error::Error + 'static>>,
    {
        match optional_config_filepath {
            Some(config_filepath) => self.try_filepath::<D>(config_filepath),
            None => Err(Error::FindOptionalConfigFile {
                optional_path: None,
            }),
        }
    }

    /// Method to call to notify/record that the configuration has been loaded
    /// from any source (for example, through environment variables, through a
    /// config filepath, through a different config struct, etc.)
    fn set_loaded(&mut self);

    /// Method to call to determine if the configuration has already been
    /// loaded at least once from any source (for example, through environment
    /// variables, through a config filepath, through a different
    /// config struct, etc.)
    fn is_loaded(&self) -> bool;

    /// Call to ensure that the configuration struct is assigned default values
    /// if loading fields was not successful from all sources tried (for
    /// example, through environment variables, through a config filepath,
    /// through a different config struct, etc.)
    fn ensure_loaded(&mut self) -> &mut Self {
        if !self.is_loaded() {
            *self = Self::default();
        }

        self
    }
}

#[cfg(feature = "serde")]
pub trait ConfigDeserialize<'de, C>
where
    C: Configuration + 'de,
{
    type Error;

    fn try_config_from_reader(reader: impl std::io::Read) -> Result<C, Self::Error>;

    fn try_config_from_string(string: &'de str) -> Result<C, Self::Error>;
}

// region: FORMAT IMPLEMENTATIONS

#[cfg(feature = "yaml")]
pub struct YamlFormat {}

#[cfg(feature = "yaml")]
impl<'de, C> ConfigDeserialize<'de, C> for YamlFormat
where
    C: for<'de1> Deserialize<'de1> + Configuration + 'de,
{
    type Error = serde_yaml::Error;

    fn try_config_from_reader(reader: impl std::io::Read) -> Result<C, Self::Error> {
        serde_yaml::from_reader(reader)
    }

    fn try_config_from_string(string: &'de str) -> Result<C, Self::Error> {
        serde_yaml::from_str(string)
    }
}

#[cfg(feature = "json")]
pub struct JsonFormat {}

#[cfg(feature = "json")]
impl<'de, C> ConfigDeserialize<'de, C> for JsonFormat
where
    C: for<'de1> Deserialize<'de1> + Configuration + 'de,
{
    type Error = serde_json::Error;

    fn try_config_from_reader(reader: impl std::io::Read) -> Result<C, Self::Error> {
        serde_json::from_reader(reader)
    }

    fn try_config_from_string(string: &'de str) -> Result<C, Self::Error> {
        serde_json::from_str(string)
    }
}

// endregion: FORMAT IMPLEMENTATIONS

// region: ERRORS

#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[non_exhaustive]
    #[snafu(display("could not find a config file at {:?}", path), visibility(pub))]
    FindConfigFile { path: PathBuf },

    #[non_exhaustive]
    #[snafu(
        display("could not find an optional config file at {:?}", optional_path),
        visibility(pub)
    )]
    FindOptionalConfigFile { optional_path: Option<PathBuf> },

    #[non_exhaustive]
    #[snafu(
        display("could not read the config file at {:?}: {source}", path),
        visibility(pub)
    )]
    ReadConfigFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[non_exhaustive]
    #[snafu(
        display(
            "could not read the optional config file at {:?}: {source}",
            optional_path
        ),
        visibility(pub)
    )]
    ReadOptionalConfigFile {
        optional_path: Option<PathBuf>,
        source: std::io::Error,
    },

    #[cfg(feature = "serde")]
    #[non_exhaustive]
    #[snafu(
        display("The config file at {:?} has incorrect format: {source}", path),
        visibility(pub)
    )]
    ParseConfigFile {
        path: PathBuf,
        source: Box<dyn std::error::Error>,
    },

    #[cfg(feature = "serde")]
    #[non_exhaustive]
    #[snafu(
        display("The config string {} has incorrect format: {source}", string),
        visibility(pub)
    )]
    ParseConfigString {
        string: String,
        source: Box<dyn std::error::Error>,
    },
}

// endregion: ERRORS

// region: IMPORTS

use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

#[cfg(feature = "serde")]
use serde::de::Deserialize;

use snafu::{self, ResultExt, Snafu};

use crate::path::ValidPath;

// endregion: IMPORTS

// region: TESTS

#[cfg(test)]
mod tests {
    #[derive(Debug, Serialize, Deserialize)]
    struct TestConfig {
        my_bool: Option<bool>,
        my_string: Option<String>,
        #[serde(skip)]
        _loaded: bool,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                my_bool: Default::default(),
                my_string: Default::default(),
                _loaded: false,
            }
        }
    }

    impl Configuration for TestConfig {
        fn new() -> Self {
            Self {
                my_bool: None,
                my_string: None,
                _loaded: false,
            }
        }

        fn config(&mut self, other: Self) -> &mut Self {
            self.my_bool = self.my_bool.take().or(other.my_bool);
            self.my_string = self.my_string.take().or(other.my_string);
            self.set_loaded();
            self
        }

        fn env(&mut self) -> &mut Self {
            todo!()
        }

        fn set_loaded(&mut self) {
            self._loaded = true;
        }

        fn is_loaded(&self) -> bool {
            self._loaded
        }
    }

    #[test]
    fn string_yaml() {
        let mut test_config = TestConfig::new();

        let test_string_1 = r#"
            my_bool: true
        "#;
        let test_string_2 = r#"
            my_string: "Hello World!"
        "#;
        let test_string_3 = r#"
            my_bool: false
        "#;
        let test_string_4 = r#"
            my_bool: false
            my_string: "Hi World!"
        "#;

        test_config.string::<YamlFormat>(test_string_1).unwrap();
        assert_eq!(test_config.my_bool, Some(true));
        assert_eq!(test_config.my_string, None);

        test_config.string::<YamlFormat>(test_string_2).unwrap();
        assert_eq!(test_config.my_bool, Some(true));
        assert_eq!(test_config.my_string, Some(String::from("Hello World!")));

        test_config.string::<YamlFormat>(test_string_3).unwrap();
        assert_eq!(test_config.my_bool, Some(true));
        assert_eq!(test_config.my_string, Some(String::from("Hello World!")));

        test_config.string::<YamlFormat>(test_string_4).unwrap();
        assert_eq!(test_config.my_bool, Some(true));
        assert_eq!(test_config.my_string, Some(String::from("Hello World!")));
    }

    // region: IMPORTS

    use serde::{Deserialize, Serialize};

    use super::*;

    // endregion: IMPORTS
}

// endregion: TESTS
