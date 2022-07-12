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


/// Represents an error, and the field the error occurred on if possible
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


#[warn(soft_unstable)]
impl<T, V> SerializeItem<Box<V>> for T where T: SerializeItem<V> {
	fn serialize(&mut self, item: Box<V>) {
		SerializeItem::<V>::serialize(self, *item);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: Box<V>) {
		SerializeItem::<V>::serialize_key(self, key, *item);
	}
}


#[warn(soft_unstable)]
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


/// Allows the serialization of serializable types as items
pub trait SerializeSerial {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T, size_type: SizeType);
	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T, size_type: SizeType);
}


/// Allows the deserialization of deserializable types from items
pub trait DeserializeSerial {
	fn deserialize<P, T: Deserialize<P>>(&mut self, size_type: SizeType) -> Result<T, DeserializationError>;
	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K, size_type: SizeType) -> Result<T, DeserializationError>;
}


/// SerializeSerial, but always uses a size type of u32
pub trait SerializeSerialDefaultSize: SerializeSerial {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T) {
		SerializeSerial::serialize(self, item, SizeType::U32);
	}
	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T) {
		SerializeSerial::serialize_key(self, key, item, SizeType::U32);
	}
}


/// DeserializeSerial, but always uses a size type of u32
pub trait DeserializeSerialDefaultSize: DeserializeSerial {
	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError> {
		DeserializeSerial::deserialize(self, SizeType::U32)
	}
	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError> {
		DeserializeSerial::deserialize_key(self, key, SizeType::U32)
	}
}


impl<T: SerializeSerial> SerializeSerialDefaultSize for T {}
impl<T: DeserializeSerial> DeserializeSerialDefaultSize for T {}


/// A standard toolset for serializing and deserializing a wide variety of types
pub trait ItemAccess:
	SerializeItem<u8> + SerializeItem<u16> + SerializeItem<String> + SerializeItemAutoSize<String> +
	DeserializeItem<u8> + DeserializeItem<u16> + DeserializeItemVarSize<String> + DeserializeItemAutoSize<String> +
	SerializeSerialDefaultSize + DeserializeSerialDefaultSize
{
	const CAN_GET_KEY: bool = false;
	fn empty() -> Self;
	fn try_get_key(&self) -> Option<&str> {
		None
	}
}


/// Allows the implementing type to be encoded in any type that implements ItemAccess
pub trait Serialize<ProfileMarker> {
	fn serialize<T: ItemAccess>(self) -> T;
}


/// Allows the implementing type to be decoded from any type that implements ItemAccess
pub trait Deserialize<ProfileMarker>: Sized {
	fn deserialize<T: ItemAccess>(data: T) -> Result<Self, DeserializationError>;
}


/// Allows the implementing type to be encoded in any type that implements ItemAccess.
/// A marshall is passed by reference. Marshalls can be used in any way that is required
pub trait MarshalledSerialize<ProfileMarker, Marshall> {
	fn serialize<T: ItemAccess>(self, marshall: &Marshall) -> T;
}


/// Allows the implementing type to be decoded from any type that implements ItemAccess
/// A marshall is passed by reference. Marshalls can be used in any way that is required.
/// Most commonly, data from the Marshall can be stored in the implementing type
pub trait MarshalledDeserialize<'a, ProfileMarker, Marshall>: Sized {
	fn deserialize<T: ItemAccess>(data: T, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
}

/// A marker trait for types that can be serialized and deserialized with the same profile,
/// without a marshall. Is automatically implemented for all appropriate types
pub trait Serde<ProfileMarker>: Serialize<ProfileMarker> + Deserialize<ProfileMarker> {}
impl<P, T: Serialize<P> + Deserialize<P>> Serde<P> for T {}

/// A marker type for serialization and deserialization of human readable data
pub struct ReadableProfile;
/// A marker type for serialization and deserialization of memory/processor efficient data
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

	#[derive(Debug)]
	struct TestStruct3<'a> {
		one: &'a TestStruct
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

	impl Serialize<ReadableProfile> for TestStruct2 {
		fn serialize<T: ItemAccess>(self) -> T {
			let mut data = T::empty();

			SerializeSerialDefaultSize::serialize_key::<ReadableProfile, _, _>(&mut data, "one", self.one);
			SerializeSerialDefaultSize::serialize_key::<ReadableProfile, _, _>(&mut data, "two", self.two);

			data
		}
	}

	impl Deserialize<ReadableProfile> for TestStruct2 {
		fn deserialize<T: ItemAccess>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				one: DeserializeSerialDefaultSize::deserialize_key::<ReadableProfile, _, _>(&mut data, "one")?,
				two: DeserializeSerialDefaultSize::deserialize_key::<ReadableProfile, _, _>(&mut data, "two")?
			})
		}
	}

	impl<'a> Serialize<ReadableProfile> for TestStruct3<'a> {
		fn serialize<T: ItemAccess>(self) -> T {
			let mut data = T::empty();
			SerializeItemAutoSize::serialize(&mut data, self.one.name.clone(), SizeType::U8);
			data
		}
	}

	impl<'a> MarshalledDeserialize<'a, ReadableProfile, TestStruct2> for TestStruct3<'a> {
		fn deserialize<T: ItemAccess>(mut data: T, marshall: &'a TestStruct2) -> Result<Self, DeserializationError> {
			let name: String = DeserializeItemAutoSize::deserialize(&mut data, SizeType::U8)?;
			if marshall.one.name == name {
				Ok(Self{ one: &marshall.one })
			} else if marshall.two.name == name {
				Ok(Self { one: &marshall.two })
			} else {
				Err(DeserializationError::from(DeserializationErrorKind::NoMatch { actual: "todo!".to_string() }))
			}
		}
	}

	impl_toml!(TestStruct, ReadableProfile);
	impl_toml!(TestStruct2, ReadableProfile);
	impl_bin!(TestStruct, EfficientProfile);
	impl_bin!(TestStruct2, ReadableProfile);

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

	#[test]
	fn test_serde_2() {
		let test = TestStruct2 {
			one: TestStruct {
				name: "a".into(),
				id: "b".into(),
				age: 0
			},
			two: TestStruct {
				name: "c".into(),
				id: "d".into(),
				age: 2
			}
		};
		let ser = test.serialize_toml();
		println!("{}", ser);
		println!("{:?}", TestStruct2::deserialize_toml(ser).unwrap());
	}

	#[test]
	fn test_serde_3() {
		let test = TestStruct2 {
			one: TestStruct {
				name: "a".into(),
				id: "b".into(),
				age: 0
			},
			two: TestStruct {
				name: "c".into(),
				id: "d".into(),
				age: 2
			}
		};
		let ser = test.serialize_bin();
		println!("{:?}", ser);
		println!("{:?}", TestStruct2::deserialize_bin(ser).unwrap());
	}

	#[test]
	fn test_serde_4() {
		let test = TestStruct2 {
			one: TestStruct {
				name: "a".into(),
				id: "b".into(),
				age: 0
			},
			two: TestStruct {
				name: "c".into(),
				id: "d".into(),
				age: 2
			}
		};
		let test2 = TestStruct3 {
			one: &test.one
		};
		let ser: Vec<u8> = test2.serialize();
		println!("{:?}", ser);
		println!("{:?}", TestStruct3::deserialize(ser, &test).unwrap());
	}
}
