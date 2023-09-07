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
        Self: Deserialize<'de>,
        D: Deserializer<'de> + From<&'de str>,
    {
        let other_config = Self::deserialize(D::from(config_string))
            .map_err(|serde_error| {
                eprintln!("{:#?}", serde_error);
                // Box::from(serde_error)
                Box::from("Hello")
            })
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
        Self: Deserialize<'de>,
        D: Deserializer<'de> + From<BufReader<File>>,
    {
        let config_filepath = config_filepath.as_ref().to_owned();
        if !config_filepath.exists() {
            Ok(self)
        } else {
            let file = File::open(config_filepath.clone()).context(ReadConfigFileSnafu {
                path: config_filepath.clone(),
            })?;
            let file_reader = BufReader::new(file);
            let other_config = Self::deserialize(D::from(file_reader))
                .map_err(|serde_error| {
                    eprintln!("{:#?}", serde_error);
                    // Box::from(serde_error)
                    Box::from("Hello")
                })
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
        Self: Deserialize<'de>,
        D: Deserializer<'de> + From<BufReader<File>> + 'de,
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
        Self: Deserialize<'de>,
        D: Deserializer<'de> + From<BufReader<File>>,
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
        Self: Deserialize<'de>,
        D: Deserializer<'de> + From<BufReader<File>>,
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

// region: REMOTE TEMPLATES

#[doc(hidden)]
#[cfg(feature = "serde")]
#[delegatable_trait_remote]
/// A copy of the remote trait signature to delegate to a newtype
trait Deserializer<'de>: Sized {
    /// The error type that can be returned if some error occurs during
    /// deserialization.
    type Error: Error;

    /// Require the `Deserializer` to figure out how to drive the visitor based
    /// on what data type is in the input.
    ///
    /// When implementing `Deserialize`, you should avoid relying on
    /// `Deserializer::deserialize_any` unless you need to be told by the
    /// Deserializer what type is in the input. Know that relying on
    /// `Deserializer::deserialize_any` means your data type will be able to
    /// deserialize from self-describing formats only, ruling out Postcard and
    /// many others.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `bool` value.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `i8` value.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `i16` value.
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `i32` value.
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `i64` value.
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `i128` value.
    ///
    /// The default behavior unconditionally returns an error.
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(Error::custom("i128 is not supported"))
    }

    /// Hint that the `Deserialize` type is expecting a `u8` value.
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `u16` value.
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `u32` value.
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `u64` value.
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an `u128` value.
    ///
    /// The default behavior unconditionally returns an error.
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = visitor;
        Err(Error::custom("u128 is not supported"))
    }

    /// Hint that the `Deserialize` type is expecting a `f32` value.
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `f64` value.
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a `char` value.
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a string value and does
    /// not benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would benefit from taking ownership of `String` data,
    /// indicate this to the `Deserializer` by using `deserialize_string`
    /// instead.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a string value and would
    /// benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would not benefit from taking ownership of `String`
    /// data, indicate that to the `Deserializer` by using `deserialize_str`
    /// instead.
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a byte array and does not
    /// benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would benefit from taking ownership of `Vec<u8>` data,
    /// indicate this to the `Deserializer` by using `deserialize_byte_buf`
    /// instead.
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a byte array and would
    /// benefit from taking ownership of buffered data owned by the
    /// `Deserializer`.
    ///
    /// If the `Visitor` would not benefit from taking ownership of `Vec<u8>`
    /// data, indicate that to the `Deserializer` by using `deserialize_bytes`
    /// instead.
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an optional value.
    ///
    /// This allows deserializers that encode an optional value as a nullable
    /// value to convert the null value into `None` and a regular value into
    /// `Some(value)`.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a unit value.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a unit struct with a
    /// particular name.
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a newtype struct with a
    /// particular name.
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a sequence of values.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a sequence of values and
    /// knows how many values there are without looking at the serialized data.
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a tuple struct with a
    /// particular name and number of fields.
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a map of key-value pairs.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting a struct with a particular
    /// name and fields.
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting an enum value with a
    /// particular name and possible variants.
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type is expecting the name of a struct
    /// field or the discriminant of an enum variant.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Hint that the `Deserialize` type needs to deserialize a value whose type
    /// doesn't matter because it is ignored.
    ///
    /// Deserializers for non-self-describing formats may not support this mode.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>;

    /// Determine whether `Deserialize` implementations should expect to
    /// deserialize their human-readable form.
    ///
    /// Some types have a human-readable form that may be somewhat expensive to
    /// construct, as well as a binary form that is compact and efficient.
    /// Generally text-based formats like JSON and YAML will prefer to use the
    /// human-readable one and binary formats like Postcard will prefer the
    /// compact one.
    ///
    /// ```edition2021
    /// # use std::ops::Add;
    /// # use std::str::FromStr;
    /// #
    /// # struct Timestamp;
    /// #
    /// # impl Timestamp {
    /// #     const EPOCH: Timestamp = Timestamp;
    /// # }
    /// #
    /// # impl FromStr for Timestamp {
    /// #     type Err = String;
    /// #     fn from_str(_: &str) -> Result<Self, Self::Err> {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// #
    /// # struct Duration;
    /// #
    /// # impl Duration {
    /// #     fn seconds(_: u64) -> Self { unimplemented!() }
    /// # }
    /// #
    /// # impl Add<Duration> for Timestamp {
    /// #     type Output = Timestamp;
    /// #     fn add(self, _: Duration) -> Self::Output {
    /// #         unimplemented!()
    /// #     }
    /// # }
    /// #
    /// use serde::de::{self, Deserialize, Deserializer};
    ///
    /// impl<'de> Deserialize<'de> for Timestamp {
    ///     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    ///     where
    ///         D: Deserializer<'de>,
    ///     {
    ///         if deserializer.is_human_readable() {
    ///             // Deserialize from a human-readable string like "2015-05-15T17:01:00Z".
    ///             let s = String::deserialize(deserializer)?;
    ///             Timestamp::from_str(&s).map_err(de::Error::custom)
    ///         } else {
    ///             // Deserialize from a compact binary representation, seconds since
    ///             // the Unix epoch.
    ///             let n = u64::deserialize(deserializer)?;
    ///             Ok(Timestamp::EPOCH + Duration::seconds(n))
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// The default implementation of this method returns `true`. Data formats
    /// may override this to `false` to request a compact form for types that
    /// support one. Note that modifying this method to change a format from
    /// human-readable to compact or vice versa should be regarded as a breaking
    /// change, as a value serialized in human-readable mode is not required to
    /// deserialize from the same data in compact mode.
    #[inline]
    fn is_human_readable(&self) -> bool {
        true
    }
}

// endregion: REMOTE TEMPLATES

// region: FORMAT IMPLEMENTATIONS

#[cfg(feature = "yaml")]
#[derive(Delegate)]
#[delegate(Deserializer<'de1>, generics = "'de1")]
pub struct YamlFormat<'de>(serde_yaml::Deserializer<'de>);

#[cfg(feature = "yaml")]
impl From<BufReader<File>> for YamlFormat<'_> {
    fn from(reader: BufReader<File>) -> Self {
        Self(serde_yaml::Deserializer::from_reader(reader))
    }
}

#[cfg(feature = "yaml")]
impl<'de> From<&'de str> for YamlFormat<'de> {
    fn from(string: &'de str) -> Self {
        Self(serde_yaml::Deserializer::from_str(string))
    }
}

#[cfg(feature = "json")]
#[derive(Delegate)]
#[delegate(Deserializer<'de1>, generics = "'de1")]
pub struct JsonFormat<R>(serde_json::Deserializer<R>);

#[cfg(feature = "json")]
impl From<BufReader<File>> for JsonFormat<serde_json::de::IoRead<BufReader<File>>> {
    fn from(reader: BufReader<File>) -> Self {
        Self(serde_json::Deserializer::from_reader(reader))
    }
}

#[cfg(feature = "json")]
impl<'a> From<&'a str> for JsonFormat<serde_json::de::StrRead<'a>> {
    fn from(string: &'a str) -> Self {
        Self(serde_json::Deserializer::from_str(string))
    }
}

#[cfg(feature = "toml")]
#[derive(Delegate)]
#[delegate(Deserializer<'de1>, generics = "'de1")]
pub struct TomlFormat<'de>(toml::Deserializer<'de>);

#[cfg(feature = "toml")]
impl<'de> From<BufReader<File>> for TomlFormat<'de> {
    fn from(mut reader: BufReader<File>) -> Self {
        let mut string = String::new();
        _ = std::io::Read::read_to_string(&mut reader, &mut string);
        Self(toml::Deserializer::new(&string))
    }
}

#[cfg(feature = "toml")]
impl<'de> From<&'de str> for TomlFormat<'de> {
    fn from(string: &'de str) -> Self {
        Self(toml::Deserializer::new(string))
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

#[cfg(feature = "serde")]
use serde::de::Visitor;

#[cfg(feature = "serde")]
use ambassador::delegatable_trait_remote;

#[cfg(feature = "serde")]
use ambassador::Delegate;

#[cfg(feature = "serde")]
use serde::Deserializer;

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
