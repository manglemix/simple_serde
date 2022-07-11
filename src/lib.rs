#[cfg(feature = "json")]
extern crate json as extern_json;
#[cfg(feature = "toml")]
extern crate toml as extern_toml;

use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::string::FromUtf8Error;

#[cfg(feature = "json")]
use extern_json::Error as JSONError;
#[cfg(feature = "toml")]
use extern_toml::de::Error as TOMLError;

#[cfg(feature = "toml")]
pub mod toml;
#[cfg(feature = "bin")]
pub mod bin;
// #[cfg(feature = "json")]
// pub mod json;

#[derive(Debug, Copy, Clone)]
pub enum SizeType {
	U8,
	U16,
	U32
}

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
	Nested(Box<DeserializationError>)
}


#[cfg(feature = "toml")]
impl From<TOMLError> for DeserializationErrorKind {
	fn from(e: TOMLError) -> Self {
		Self::TOMLError(e)
	}
}
#[cfg(feature = "json")]
impl From<JSONError> for DeserializationErrorKind {
	fn from(e: JSONError) -> Self {
		Self::JSONError(e)
	}
}
impl From<FromUtf8Error> for DeserializationErrorKind {
	fn from(e: FromUtf8Error) -> Self {
		Self::FromUTF8Error(e)
	}
}


pub struct DeserializationError {
	pub field: Option<String>,
	pub kind: DeserializationErrorKind
}


impl Debug for DeserializationError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match &self.field {
			None => write!(f, "Faced the following deserialization error: {:?}", self.kind),
			Some(x) => write!(f, "Faced the following deserialization error of field: {} => {:?}", x, self.kind)
		}
	}
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


impl From<DeserializationError> for DeserializationErrorKind {
	fn from(e: DeserializationError) -> Self {
		DeserializationErrorKind::Nested(Box::new(e))
	}
}


#[cfg(feature = "toml")]
impl From<TOMLError> for DeserializationError {
	fn from(e: TOMLError) -> Self {
		Self::from(DeserializationErrorKind::from(e))
	}
}
#[cfg(feature = "json")]
impl From<JSONError> for DeserializationError {
	fn from(e: JSONError) -> Self {
		Self::from(DeserializationErrorKind::from(e))
	}
}
impl From<FromUtf8Error> for DeserializationError {
	fn from(e: FromUtf8Error) -> Self {
		Self::from(DeserializationErrorKind::from(e))
	}
}


/// For types that can be serialized as is
pub trait SerializeItem<T> {
	fn serialize(&mut self, item: T);
	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: T);
	// fn serialize_boxed(&mut self, item: Box<T>) {
	// 	self.serialize(*item);
	// }
	// fn serialize_key_boxed<K: Borrow<str>>(&mut self, key: K, item: Box<T>) {
	// 	self.serialize_key(key, *item);
	// }
	// fn serialize_option(&mut self, item: Option<T>) {
	// 	match item {
	// 		Some(x) => self.se
	// 	}
	// }
}


/// For types that must be serialized alongside their size
pub trait SerializeItemAutoSize<T> {
	fn serialize(&mut self, item: T, size_type: SizeType);
	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: T, size_type: SizeType);
}


/// For types that can be deserialized as is.
/// In other words, the true size of the type is fixed
pub trait DeserializeItem<T> {
	fn deserialize(&mut self) -> Result<T, DeserializationErrorKind>;
	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationErrorKind>;
}


/// For types whose sizes can vary.
/// This will look for a number representing the size of the item to deserialize.
/// For use with SerializeItemAutoSize
pub trait DeserializeItemAutoSize<T> {
	fn deserialize(&mut self, size_type: SizeType) -> Result<T, DeserializationErrorKind>;
	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, size_type: SizeType) -> Result<T, DeserializationErrorKind>;
}


/// For deserializing types with a given size
pub trait DeserializeItemVarSize<T> {
	fn deserialize(&mut self, size: usize) -> Result<T, DeserializationErrorKind>;
	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, size: usize) -> Result<T, DeserializationErrorKind>;
}


impl<T, V> SerializeItem<Box<V>> for T where T: SerializeItem<V> {
	fn serialize(&mut self, item: Box<V>) {
		SerializeItem::<V>::serialize(self, *item);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: Box<V>) {
		SerializeItem::<V>::serialize_key(self, key, *item);
	}
}


impl<T, V> SerializeItem<Option<V>> for T where T: SerializeItem<V> {
	fn serialize(&mut self, item: Option<V>) {
		match item {
			Some(x) => SerializeItem::<V>::serialize(self, x),
			None => {}
		}
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: Option<V>) {
		match item {
			Some(x) => SerializeItem::<V>::serialize_key(self, key, x),
			None => {}
		}
	}
}


/// A standard toolset for serializing and deserializing a wide variety of types
pub trait ItemAccess:
	SerializeItem<u8> + SerializeItem<u16> + SerializeItem<String> + SerializeItemAutoSize<String> +
	DeserializeItem<u8> + DeserializeItem<u16> + DeserializeItemVarSize<String> + DeserializeItemAutoSize<String>
{
	const CAN_GET_KEY: bool = false;
	fn empty() -> Self;
	fn try_get_key(&self) -> Option<&str> {
		None
	}
}


pub trait Serialize<ProfileMarker> {
	fn serialize<T: ItemAccess>(self) -> T;
}


pub trait Deserialize<ProfileMarker>: Sized {
	fn deserialize<T: ItemAccess>(data: T) -> Result<Self, DeserializationError>;
}

/// A marker trait for types that can be serialized and deserialized with the same profile
pub trait Serde<ProfileMarker>: Serialize<ProfileMarker> + Deserialize<ProfileMarker> {}
impl<P, T: Serialize<P> + Deserialize<P>> Serde<P> for T {}

pub struct ReadableProfile;
pub struct EfficientProfile;


#[cfg(test)]
mod tests {
	use crate::bin::BinSerde;
	use crate::toml::TOMLSerde;
	use super::*;

    #[derive(Debug)]
	struct TestStruct {
		name: String,
		id: String,
		age: u16
	}

	#[derive(Debug)]
	struct TestStruct2 {
		one: TestStruct,
		two: TestStruct
	}

	impl Serialize<ReadableProfile> for TestStruct {
		fn serialize<T: ItemAccess>(self) -> T {
			let mut data = T::empty();

			SerializeItemAutoSize::serialize_key(&mut data, "name", self.name, SizeType::U8);
			SerializeItemAutoSize::serialize_key(&mut data, "id", self.id, SizeType::U8);
			SerializeItem::serialize_key(&mut data, "age", self.age);

			data
		}
	}

	impl Deserialize<ReadableProfile> for TestStruct {
		fn deserialize<T: ItemAccess>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				name: DeserializeItemAutoSize::deserialize_key(&mut data, "name", SizeType::U8)?,
				id: DeserializeItemAutoSize::deserialize_key(&mut data, "id", SizeType::U8)?,
				age: DeserializeItem::deserialize_key(&mut data, "age")?
			})
		}
	}

	impl Serialize<EfficientProfile> for TestStruct {
		fn serialize<T: ItemAccess>(self) -> T {
			let mut data = T::empty();

			SerializeItemAutoSize::serialize(&mut data, self.name, SizeType::U8);
			SerializeItemAutoSize::serialize(&mut data, self.id, SizeType::U8);
			SerializeItem::serialize(&mut data, self.age);

			data
		}
	}

	impl Deserialize<EfficientProfile> for TestStruct {
		fn deserialize<T: ItemAccess>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				name: DeserializeItemAutoSize::deserialize(&mut data, SizeType::U8)?,
				id: DeserializeItemAutoSize::deserialize(&mut data, SizeType::U8)?,
				age: DeserializeItem::deserialize(&mut data)?
			})
		}
	}

	impl_toml!(TestStruct, ReadableProfile);
	impl_bin!(TestStruct, EfficientProfile);

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
		println!("{:?}\n", ser);
		println!("{:?}", TestStruct::deserialize_bin(ser).unwrap());
	}

	// #[test]
	// fn test_serde_2() {
	// 	let test = TestStruct2 {
	// 		one: TestStruct {
	// 			name: "a".into(),
	// 			id: "b".into(),
	// 			age: 0
	// 		},
	// 		two: TestStruct {
	// 			name: "c".into(),
	// 			id: "d".into(),
	// 			age: 2
	// 		}
	// 	};
	// 	let ser = test.serialize_toml();
	// 	println!("{}", ser);
	// 	println!("{:?}", TestStruct2::deserialize_toml(ser).unwrap());
	// }
}
