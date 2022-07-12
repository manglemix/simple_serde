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
#[cfg(feature = "json")]
pub mod json;

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


pub enum SimpleNumber {
	I64(i64),
	F64(f64)
}


pub trait NumberType: Sized {
	fn to_simple(self) -> SimpleNumber;
	#[cfg(feature = "bin")]
	fn from_bin(bin: &mut Vec<u8>) -> Result<Self, DeserializationErrorKind>;
	fn from_i64(int: i64) -> Option<Self>;
	fn from_f64(float: f64) -> Option<Self>;
}


macro_rules! serial_int {
    ($type: ty) => {
impl NumberType for $type {
	fn to_simple(self) -> SimpleNumber {
		SimpleNumber::I64(self as i64)
	}
	#[cfg(feature = "bin")]
	fn from_bin(bin:&mut Vec<u8>) -> Result<Self, DeserializationErrorKind> {
		todo!()
	}
	fn from_i64(int: i64) -> Option<Self> {
		Some(int as $type)
	}
	fn from_f64(_float: f64) -> Option<Self> {
		None
	}
}
impl Serialize for $type {
	fn serialize<T: Serializer>(self) -> T {
		let mut data = T::empty();
		data.serialize_num(self);
		data
	}
}
impl Deserialize for $type {
	fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
		data.deserialize_num()
	}
}
	};
}

serial_int!(u8);
serial_int!(u16);
serial_int!(u32);
serial_int!(u64);
serial_int!(i8);
serial_int!(i16);
serial_int!(i32);
serial_int!(i64);


macro_rules! serial_string {
    ($type: ty) => {
impl Serialize for $type {
	fn serialize<T: Serializer>(self) -> T {
		let mut data = T::empty();
		data.serialize_string(self);
		data
	}
}
	};
}

serial_string!(String);
serial_string!(&str);

impl Deserialize for String {
	fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
		data.deserialize_string()
	}
}


macro_rules! from_string {
    ($type: ty) => {
impl Deserialize for $type {
	fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
		data.deserialize_string().and_then(|x| { Ok(<$type>::from(x)) })
	}
}
	};
}


from_string!(std::path::PathBuf);


/// A standard toolset for serializing and deserializing a wide variety of types
pub trait PrimitiveSerializer {
	fn serialize_num<T: NumberType>(&mut self, num: T);
	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError>;

	fn serialize_string<T: Into<String>>(&mut self, string: T);
	fn deserialize_string(&mut self) -> Result<String, DeserializationError> {
		self.deserialize_string_auto_size(SizeType::U32)
	}
	fn deserialize_string_sized(&mut self, size: usize) -> Result<String, DeserializationError>;
	fn deserialize_string_auto_size(&mut self, size_type: SizeType) -> Result<String, DeserializationError>;
}


pub trait Serializer: PrimitiveSerializer {
	fn empty() -> Self;
	fn serialize<P, T: Serialize<P>>(&mut self, item: T);
	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T);
	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError>;
	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError>;
}


/// Allows the implementing type to be encoded in any type that implements ItemAccess
pub trait Serialize<ProfileMarker= NaturalProfile> {
	fn serialize<T: Serializer>(self) -> T;
}


/// Allows the implementing type to be decoded from any type that implements ItemAccess
pub trait Deserialize<ProfileMarker= NaturalProfile>: Sized {
	fn deserialize<T: Serializer>(data: T) -> Result<Self, DeserializationError>;
}


// /// Allows the implementing type to be encoded in any type that implements ItemAccess.
// /// A marshall is passed by reference. Marshalls can be used in any way that is required
// pub trait MarshalledSerialize<ProfileMarker, Marshall> {
// 	fn serialize<T: ItemAccess>(self, marshall: &Marshall) -> T;
// }
//
//
// /// Allows the implementing type to be decoded from any type that implements ItemAccess
// /// A marshall is passed by reference. Marshalls can be used in any way that is required.
// /// Most commonly, data from the Marshall can be stored in the implementing type
// pub trait MarshalledDeserialize<'a, ProfileMarker, Marshall>: Sized {
// 	fn deserialize<T: ItemAccess>(data: T, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
// }

/// A marker trait for types that can be serialized and deserialized with the same profile,
/// without a marshall. Is automatically implemented for all appropriate types
pub trait Serde<ProfileMarker>: Serialize<ProfileMarker> + Deserialize<ProfileMarker> {}
impl<P, T: Serialize<P> + Deserialize<P>> Serde<P> for T {}

/// A marker type for serialization and deserialization of data.
/// the default way that a type should be serialized. Mainly used for common types like ints, String, etc
pub struct NaturalProfile;
/// A marker type for serialization and deserialization of human readable data
pub struct ReadableProfile;
/// A marker type for serialization and deserialization of memory/processor efficient data
pub struct EfficientProfile;


#[cfg(test)]
mod tests {
	#[cfg(feature = "bin")]
	use crate::bin::{BinSerde, impl_bin};
	use crate::{DeserializationError, Deserialize, EfficientProfile, ReadableProfile, Serialize, Serializer};
	#[cfg(feature = "toml")]
	use crate::toml::{TOMLSerde, impl_toml};

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
		fn serialize<T: Serializer>(self) -> T {
			let mut data = T::empty();

			data.serialize_key("name", self.name);
			data.serialize_key("age", self.age);
			data.serialize_key("id", self.id);

			data
		}
	}

	impl Deserialize<ReadableProfile> for TestStruct {
		fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				name: data.deserialize_key("name")?,
				id: data.deserialize_key("id")?,
				age: data.deserialize_key("age")?
			})
		}
	}

	impl Serialize<EfficientProfile> for TestStruct {
		fn serialize<T: Serializer>(self) -> T {
			let mut data = T::empty();

			data.serialize(self.name);
			data.serialize(self.age);
			data.serialize(self.id);

			data
		}
	}

	impl Deserialize<EfficientProfile> for TestStruct {
		fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				name: data.deserialize()?,
				id: data.deserialize()?,
				age: data.deserialize()?
			})
		}
	}

	impl Serialize<ReadableProfile> for TestStruct2 {
		fn serialize<T: Serializer>(self) -> T {
			let mut data = T::empty();

			data.serialize_key::<ReadableProfile, _, _>("one", self.one);
			data.serialize_key::<ReadableProfile, _, _>("two", self.two);

			data
		}
	}

	impl Deserialize<ReadableProfile> for TestStruct2 {
		fn deserialize<T: Serializer>(mut data: T) -> Result<Self, DeserializationError> {
			Ok(Self {
				one: data.deserialize_key::<ReadableProfile, _, _>("one")?,
				two: data.deserialize_key::<ReadableProfile, _, _>("two")?
			})
		}
	}

	impl<'a> Serialize<ReadableProfile> for TestStruct3<'a> {
		fn serialize<T: Serializer>(self) -> T {
			let mut data = T::empty();
			data.serialize_key("name", self.one.name.clone());
			data
		}
	}

	// impl<'a> MarshalledDeserialize<'a, ReadableProfile, TestStruct2> for TestStruct3<'a> {
	// 	fn deserialize<T: Serializer>(mut data: T, marshall: &'a TestStruct2) -> Result<Self, DeserializationError> {
	// 		let name: String = DeserializeItemAutoSize::deserialize(&mut data, SizeType::U8)?;
	// 		if marshall.one.name == name {
	// 			Ok(Self{ one: &marshall.one })
	// 		} else if marshall.two.name == name {
	// 			Ok(Self { one: &marshall.two })
	// 		} else {
	// 			Err(DeserializationError::from(DeserializationErrorKind::NoMatch { actual: "todo!".to_string() }))
	// 		}
	// 	}
	// }

	#[cfg(feature = "toml")]
	impl_toml!(TestStruct, ReadableProfile);
	#[cfg(feature = "toml")]
	impl_toml!(TestStruct2, ReadableProfile);
	#[cfg(feature = "bin")]
	impl_bin!(TestStruct, EfficientProfile);
	#[cfg(feature = "bin")]
	impl_bin!(TestStruct2, ReadableProfile);

	#[cfg(feature = "toml")]
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

	#[cfg(feature = "bin")]
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

	#[cfg(feature = "toml")]
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

	#[cfg(feature = "bin")]
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

	#[cfg(feature = "bin")]
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
