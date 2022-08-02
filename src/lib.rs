use std::borrow::{Borrow, BorrowMut};
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::string::FromUtf8Error;

#[cfg(feature = "bin")]
pub mod bin;
pub mod common;
mod primitives;
#[cfg(feature = "text")]
pub mod text;

pub use primitives::{NumberType};

pub mod prelude {
	pub use crate::{impl_key_serde, impl_key_ser, impl_key_deser, Serialize, Deserialize, Serializer, DeserializationError, ReadableProfile, EfficientProfile};
}

#[cfg(feature = "text")]
pub use text::{json_prelude, toml_prelude, toml, json};
#[cfg(feature = "bin")]
pub use bin::prelude as bin_prelude;

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
	Nested(Box<DeserializationError>),
	InvalidFormat {
		reason: String
	}
}


// impl DeserializationErrorKind {
// 	pub fn invalid_format<T: ToString>(reason: T) -> Self {
// 		Self::InvalidFormat { reason: reason.to_string() }
// 	}
// }


impl From<FromUtf8Error> for DeserializationErrorKind {
	fn from(e: FromUtf8Error) -> Self {
		Self::FromUTF8Error(e)
	}
}


trait DeserializationResult {
	type Output;
	fn set_field<T: ToString>(self, field: T) -> Self::Output;
	fn no_field(self) -> Self::Output;
}


impl<T> DeserializationResult for Result<T, DeserializationErrorKind> {
	type Output = Result<T, DeserializationError>;

	fn set_field<K: ToString>(self, field: K) -> Self::Output {
		match self {
			Ok(x) => Ok(x),
			Err(e) => Err(DeserializationError::new(field, e))
		}
	}

	fn no_field(self) -> Self::Output {
		self.map_err(DeserializationError::new_kind)
	}
}


impl<T> DeserializationResult for Result<T, DeserializationError> {
	type Output = Result<T, DeserializationError>;

	fn set_field<K: ToString>(self, field: K) -> Self::Output {
		match self {
			Ok(x) => Ok(x),
			Err(e) => Err(e.set_field(field))
		}
	}

	fn no_field(self) -> Self::Output {
		self.map_err(DeserializationError::new_kind)
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
			Some(x) => write!(f, "Faced the following deserialization error on field: {} => {:?}", x, self.kind)
		}
	}
}


impl DeserializationError {
	const EOF: Self = Self { field: None, kind: DeserializationErrorKind::UnexpectedEOF };

	pub fn new_kind<E: Into<DeserializationErrorKind>>(error: E) -> Self {
		Self { field: None, kind: error.into() }
	}
	pub fn new<T: ToString, E: Into<DeserializationErrorKind>>(field: T, error: E) -> Self {
		Self { field: Some(field.to_string()), kind: error.into() }
	}
	pub fn missing_field<T: ToString>(field: T) -> Self {
		Self { field: Some(field.to_string()), kind: DeserializationErrorKind::MissingField }
	}
	pub fn invalid_format<T: ToString>(reason: T) -> Self {
		Self { field: None, kind: DeserializationErrorKind::InvalidFormat { reason: reason.to_string() } }
	}
	pub fn set_field<T: ToString>(mut self, field: T) -> Self {
		self.field = Some(field.to_string());
		self
	}
	pub fn nest(self) -> Self {
		Self::new_kind(DeserializationErrorKind::from(self))
	}
}


impl From<DeserializationError> for DeserializationErrorKind {
	fn from(e: DeserializationError) -> Self {
		DeserializationErrorKind::Nested(Box::new(e))
	}
}


/// A standard toolset for serializing and deserializing a wide variety of types
pub trait PrimitiveSerializer {
	fn serialize_bool(&mut self, boolean: bool);
	fn deserialize_bool(&mut self) -> Result<bool, DeserializationError>;

	fn serialize_num<T: NumberType>(&mut self, num: T);
	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError>;

	fn serialize_string<T: Into<String>>(&mut self, string: T);
	fn deserialize_string(&mut self) -> Result<String, DeserializationError>;

	fn serialize_bytes<T: Into<VecDeque<u8>>>(&mut self, bytes: T);
	fn deserialize_bytes<T: FromIterator<u8>>(&mut self) -> Result<T, DeserializationError>;
}


/// A trait for data structures that can serialize or deserialize into other types that implement Serialize or Deserialize respectively
pub trait Serializer: PrimitiveSerializer + Debug {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T);
	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T);
	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError>;
	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError>;
	fn deserialize_key_or<P, T: Deserialize<P>, K: Borrow<str>, V: Into<T>>(&mut self, key: K, or: V) -> Result<T, DeserializationError> {
		self.deserialize_key(key).or_else(|e| match &e.kind {
			DeserializationErrorKind::MissingField => Ok(or.into()),
			_ => Err(e)
		})
	}
	fn deserialize_key_or_else<P, T, K, F>(&mut self, key: K, or: F) -> Result<T, DeserializationError>
		where
			T: Deserialize<P>, K: Borrow<str>,
			F: FnOnce() -> T
	{
		self.deserialize_key(key).or_else(|e| match &e.kind {
			DeserializationErrorKind::MissingField => Ok(or()),
			_ => Err(e)
		})
	}
	/// Try to get a key if it is the next item
	fn try_get_key(&mut self) -> Option<String>;
}


/// Allows the implementing type to be encoded in any type that implements ItemAccess
pub trait Serialize<ProfileMarker = NaturalProfile> {
	fn serialize<T: Serializer>(self, data: &mut T);
}


/// Allows the implementing type to be decoded from any type that implements ItemAccess
pub trait Deserialize<ProfileMarker = NaturalProfile>: Sized {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError>;
}


/// Allows the implementing type to be encoded in any type that implements ItemAccess.
/// A marshall is passed by reference. Marshalls can be used in any way that is required
pub trait MarshalledSerialize<ProfileMarker, Marshall> {
	fn serialize<T: Serializer>(self, data: &mut T, marshall: &Marshall);
}


/// Allows the implementing type to be decoded from any type that implements ItemAccess
/// A marshall is passed by reference. Marshalls can be used in any way that is required.
/// Most commonly, data from the Marshall can be stored in the implementing type
pub trait MarshalledDeserialize<'a, ProfileMarker, Marshall>: Sized {
	fn deserialize<T: Serializer>(data: &mut T, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
}

/// A marker trait for types that can be serialized and deserialized with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait Serde<ProfileMarker>: Serialize<ProfileMarker> + Deserialize<ProfileMarker> {}
impl<P, T: Serialize<P> + Deserialize<P>> Serde<P> for T {}

/// A marker trait for types that can be serialized and deserialized with the same profile,
/// and the same type of marshall. Is automatically implemented on all appropriate types
pub trait MarshalledSerde<'a, ProfileMarker, Marshall>: MarshalledSerialize<ProfileMarker, Marshall> + MarshalledDeserialize<'a, ProfileMarker, Marshall> {}
impl<'a, P, M, T: MarshalledSerialize<P, M> + MarshalledDeserialize<'a, P, M>> MarshalledSerde<'a, P, M> for T {}

/// A marker type for serialization and deserialization of data.
/// the default way that a type should be serialized. Mainly used for common types like ints, String, etc
pub struct NaturalProfile;
/// A marker type for serialization and deserialization of human readable data
pub struct ReadableProfile;
/// A marker type for serialization and deserialization of memory/processor efficient data
pub struct EfficientProfile;


impl<P, S: Serialize<P>> Serialize<P> for Box<S> {
	fn serialize<T: Serializer>(self, data: &mut T) {
		data.serialize(*self);
	}
}


#[macro_export]
macro_rules! impl_key_serde {
    ($name: ty, $profile: ty, $($field: ident),*) => {
		impl_key_ser!($name, $profile, $($field),*);
		impl_key_deser!($name, $profile, $($field),*);
	};
}

#[macro_export]
macro_rules! impl_key_ser {
    ($name: ty, $profile: ty, $($field: ident),*) => {
		impl Serialize<$profile> for $name {
			fn serialize<T: Serializer>(self, data: &mut T) {
				$(data.serialize_key(stringify!($field), self.$field);)*
			}
		}
	};
}

#[macro_export]
macro_rules! impl_key_deser {
    ($name: ty, $profile: ty, $($field: ident),*) => {
		impl Deserialize<$profile> for $name {
			fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
				Ok(Self {
					$($field: data.deserialize_key(stringify!($field))?,)*
				})
			}
		}
	};
}


#[cfg(test)]
mod tests {
	use std::collections::VecDeque;
	#[cfg(feature = "bin")]
	use crate::bin::{BinSerialize, BinDeserialize};
	#[cfg(feature = "bin")]
	use crate::impl_bin;
	use crate::{prelude::*, DeserializationErrorKind, MarshalledDeserialize};
	#[cfg(feature = "text")]
	use crate::text::{toml_prelude::*, json_prelude::*};

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

	impl_key_serde!(TestStruct, ReadableProfile, name, id, age);

	impl Serialize<EfficientProfile> for TestStruct {
		fn serialize<T: Serializer>(self, data: &mut T) {
			data.serialize(self.name);
			data.serialize(self.age);
			data.serialize(self.id);
		}
	}

	impl Deserialize<EfficientProfile> for TestStruct {
		fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
			Ok(Self {
				name: data.deserialize()?,
				age: data.deserialize()?,
				id: data.deserialize()?,
			})
		}
	}

	impl Serialize<ReadableProfile> for TestStruct2 {
		fn serialize<T: Serializer>(self, data: &mut T) {
			data.serialize_key::<ReadableProfile, _, _>("one", self.one);
			data.serialize_key::<ReadableProfile, _, _>("two", self.two);
		}
	}

	impl Deserialize<ReadableProfile> for TestStruct2 {
		fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
			Ok(Self {
				one: data.deserialize_key::<ReadableProfile, _, _>("one")?,
				two: data.deserialize_key::<ReadableProfile, _, _>("two")?
			})
		}
	}

	impl<'a> Serialize<ReadableProfile> for TestStruct3<'a> {
		fn serialize<T: Serializer>(self, data: &mut T) {
			data.serialize_key("name", self.one.name.clone());
		}
	}

	impl<'a> MarshalledDeserialize<'a, ReadableProfile, TestStruct2> for TestStruct3<'a> {
		fn deserialize<T: Serializer>(data: &mut T, marshall: &'a TestStruct2) -> Result<Self, DeserializationError> {
			let name: String = data.deserialize_key("name")?;
			if marshall.one.name == name {
				Ok(Self{ one: &marshall.one })
			} else if marshall.two.name == name {
				Ok(Self { one: &marshall.two })
			} else {
				Err(DeserializationError::new_kind(DeserializationErrorKind::NoMatch { actual: "todo!".to_string() }))
			}
		}
	}

	#[cfg(feature = "text")]
	impl_toml!(TestStruct, ReadableProfile);
	#[cfg(feature = "text")]
	impl_json!(TestStruct, ReadableProfile);
	#[cfg(feature = "text")]
	impl_toml!(TestStruct2, ReadableProfile);
	#[cfg(feature = "bin")]
	impl_bin!(TestStruct, EfficientProfile);
	#[cfg(feature = "bin")]
	impl_bin!(TestStruct2, ReadableProfile);
	#[cfg(feature = "text")]
	impl_json!(TestStruct2, ReadableProfile);

	#[cfg(feature = "text")]
	#[test]
	fn test_serde_0() {
		let test = TestStruct {
			name: "lmf".into(),
			id: "55".into(),
			age: 22
		};
		let ser = test.serialize_toml();
		// let ser = "id = \"55\"
		// age = 22
		// name = \"lmf\"".to_string();
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

	#[cfg(feature = "text")]
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
		let ser = test.serialize_json();
		println!("{}", ser);
		println!("{:?}", TestStruct2::deserialize_json(ser).unwrap());
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
		let mut ser: VecDeque<u8> = VecDeque::new();
		test2.serialize(&mut ser);
		println!("{:?}", ser);
		println!("{:?}", TestStruct3::deserialize(&mut ser, &test).unwrap());
	}

	#[cfg(feature = "text")]
	#[test]
	fn test_serde_5() {
		let test = TestStruct {
			name: "lmf".into(),
			id: "55".into(),
			age: 22
		};
		let ser = test.serialize_json();
		// let ser = "id = \"55\"
		// age = 22
		// name = \"lmf\"".to_string();
		println!("{}", ser);
		println!("{:?}", TestStruct::deserialize_json(ser).unwrap());
	}
}
