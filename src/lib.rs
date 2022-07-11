#[cfg(feature = "json")]
extern crate json as extern_json;
#[cfg(feature = "toml")]
extern crate toml as extern_toml;

use std::mem::MaybeUninit;
use std::string::FromUtf8Error;

#[cfg(feature = "json")]
use extern_json::Error as JSONError;
#[cfg(feature = "toml")]
use extern_toml::de::Error as TOMLError;

use map::SerdeMapItem;
use crate::array::{ArrayAccess, ArrayDataContainer};

use crate::map::{MapAccess, MapDataContainer};

#[cfg(feature = "toml")]
pub mod toml;
pub mod map;
pub mod array;
#[cfg(feature = "bin")]
mod bin;

/// An error that can occur when trying to deserialize data
#[derive(Debug)]
pub enum DeserializationErrorKind {
	/// An expected field could not be found
	/// Contains the field name
	MissingField,
	/// An expected field has an unexpected data type
	InvalidType {
		/// The expected type of the field
		expected: &'static str,
		/// The actual type of the field
		actual: &'static str,
	},
	/// An expected field does not contain any of the expected data
	NoMatch {
		/// The actual data contained in the field
		actual: String,
	},
	/// An error creating a utf-8 string from binary
	FromUTF8Error(FromUtf8Error),
	/// The data we are deserializing from is too short
	UnexpectedEOF,
	#[cfg(feature = "toml")]
	/// An error occurred while parsing TOML formatted data
	TOMLError(TOMLError),
	#[cfg(feature = "json")]
	/// An error occurred while parsing JSON formatted data
	JSONError(JSONError),
}


#[cfg(feature = "toml")]
impl From<TOMLError> for DeserializationErrorKind {
	fn from(e: TOMLError) -> Self {
		Self::TOMLError(e)
	}
}
impl From<FromUtf8Error> for DeserializationErrorKind {
	fn from(e: FromUtf8Error) -> Self {
		Self::FromUTF8Error(e)
	}
}


#[derive(Debug)]
pub struct DeserializationError {
	pub field: Option<String>,
	pub kind: DeserializationErrorKind
}


impl DeserializationError {
	pub fn new<T: Into<String>, E: Into<DeserializationErrorKind>>(field: T, error: E) -> Self {
		Self { field: Some(field.into()), kind: error.into() }
	}
}


impl From<DeserializationErrorKind> for DeserializationError {
	fn from(kind: DeserializationErrorKind) -> Self {
		DeserializationError { field: None, kind }
	}
}


#[cfg(feature = "toml")]
impl From<TOMLError> for DeserializationError {
	fn from(e: TOMLError) -> Self {
		Self::from(DeserializationErrorKind::from(e))
	}
}


#[macro_export]
macro_rules! map_serde {
    ($self: expr, $data: expr, $name: ident) => {
        $data.serde(stringify!($name), &mut $self.$name)?;
    };
}


#[macro_export]
macro_rules! array_serde {
    ($self: expr, $data: expr, $name: ident, U8) => {
        SerdeArrayItemUnsized::serde($data, &mut $self.$name, SizeType::U8)?;
    };
    ($self: expr, $data: expr, $name: ident, U16) => {
        SerdeArrayItemUnsized::serde($data, &mut $self.$name, SizeType::U16)?;
    };
    ($self: expr, $data: expr, $name: ident, U32) => {
        SerdeArrayItemUnsized::serde($data, &mut $self.$name, SizeType::U32)?;
    };
    ($self: expr, $data: expr, $name: ident, $size: expr) => {
        SerdeArrayItemUnsized::serde_sized($data, &mut $self.$name, $size)?;
    };
    ($self: expr, $data: expr, $name: ident) => {
        SerdeArrayItemSized::serde($data, &mut $self.$name)?;
    };
}


/// Initializes the implementing type with uninitialized memory.
/// This is fast, and safe IF all values and references are initialized properly after creation.
/// If the implementing type implements Default, Initialize is implemented safely using default.
/// Only implement this trait if performance is important, and you can absolutely guarantee that all
/// uninitialized memory is replaced with valid data
pub unsafe trait Initialize: Sized {
	/// Creates an instance of Self.
	/// The default implementation is very unsafe, so
	/// ensure that all data in Self is initialized to valid data
	#[inline]
	unsafe fn unsafe_init() -> Self {
		MaybeUninit::zeroed().assume_init()
	}
}


unsafe impl<T: Default> Initialize for T {
	#[inline]
	unsafe fn unsafe_init() -> Self {
		Self::default()
	}
}


pub trait MappedSerde<ProfileMarker>: Initialize {
	fn serde<T: MapAccess>(&mut self, data: &mut MapDataContainer<T>) -> Result<(), DeserializationError>;
	fn serialize<T: MapAccess>(mut self, inner_data: T) -> T {
		let mut data = MapDataContainer {
			serializing: true,
			data: inner_data
		};
		self.serde(&mut data).expect("Faced an unexpected error during serialization. This should not be the case");
		data.data
	}
	fn deserialize<T: MapAccess>(inner_data: T) -> Result<Self, DeserializationError> {
		unsafe {
			let mut data = MapDataContainer {
				serializing: false,
				data: inner_data
			};
			let mut instance = Self::unsafe_init();
			instance.serde(&mut data)?;
			Ok(instance)
		}
	}
}


pub trait ArraySerde<ProfileMarker>: Initialize {
	fn serde<T: ArrayAccess>(&mut self, data: &mut ArrayDataContainer<T>) -> Result<(), DeserializationError>;
	fn serialize<T: ArrayAccess>(mut self, inner_data: T) -> T {
		let mut data = ArrayDataContainer {
			serializing: true,
			data: inner_data
		};
		self.serde(&mut data).expect("Faced an unexpected error during serialization. This should not be the case");
		data.data
	}
	fn deserialize<T: ArrayAccess>(inner_data: T) -> Result<Self, DeserializationError> {
		unsafe {
			let mut data = ArrayDataContainer {
				serializing: false,
				data: inner_data
			};
			let mut instance = Self::unsafe_init();
			instance.serde(&mut data)?;
			Ok(instance)
		}
	}
}


pub struct ReadableProfile;

pub struct EfficientProfile;


#[cfg(test)]
mod tests {
	use crate::array::{SerdeArrayItemSized, SerdeArrayItemUnsized, SizeType};
	use crate::bin::BinSerde;
	use crate::toml::TOMLSerde;

    use super::*;

    #[derive(Debug)]
	struct TestStruct {
		name: String,
		id: String,
		age: u16
	}

	unsafe impl Initialize for TestStruct {}

	impl MappedSerde<ReadableProfile> for TestStruct {
		fn serde<T: MapAccess>(&mut self, data: &mut MapDataContainer<T>) -> Result<(), DeserializationError> {
			map_serde!(self, data, name);
			map_serde!(self, data, id);
			map_serde!(self, data, age);
			Ok(())
		}
	}

	impl ArraySerde<EfficientProfile> for TestStruct {
		fn serde<T: ArrayAccess>(&mut self, data: &mut ArrayDataContainer<T>) -> Result<(), DeserializationError> {
			array_serde!(self, data, name, U8);
			array_serde!(self, data, id, 2);
			array_serde!(self, data, age);
			Ok(())
		}
	}

	impl TOMLSerde<ReadableProfile> for TestStruct {}
	impl BinSerde<EfficientProfile> for TestStruct {}

	#[test]
	fn test_serde_0() {
		let test = TestStruct {
			name: "lmf".into(),
			id: "55".into(),
			age: 22
		};
		let ser = test.serialize_toml();
		println!("{}", ser);
		println!("{:?}", TestStruct::deserialize_toml(ser).unwrap());
	}

	#[test]
	fn test_serde_1() {
		let test = TestStruct {
			name: "lmf".into(),
			id: "55".into(),
			age: 22
		};
		let ser = test.serialize_bin();
		println!("{:?}", ser);
		println!("{:?}", TestStruct::deserialize_bin(ser).unwrap());
	}
}
