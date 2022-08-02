use std::collections::VecDeque;

use crate::text::TextRepr;

use super::{DeserializationError, Deserialize, Serialize, Serializer};
#[cfg(feature = "bin")]
use super::{bin, DeserializationErrorKind};

/// Trait for types that are either integers or floats
pub trait NumberType: Sized {
	#[cfg(feature = "text")]
	fn to_text(self) -> TextRepr;
	#[cfg(feature = "text")]
	fn from_i64(int: i64) -> Option<Self>;
	#[cfg(feature = "text")]
	fn from_f64(float: f64) -> Option<Self>;
	#[cfg(feature = "bin")]
	fn from_bin(bin: &mut VecDeque<u8>) -> Result<Self, DeserializationErrorKind>;
	#[cfg(feature = "bin")]
	fn to_bin(self) -> VecDeque<u8>;
}


macro_rules! impl_serde_number {
    ($type: ty) => {
impl Serialize for $type {
	fn serialize<T: Serializer>(self, data: &mut T) {
		data.serialize_num(self);
	}
}
impl Deserialize for $type {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		data.deserialize_num()
	}
}
	};
}


/// Implement serialize and deserialize for integer types
macro_rules! serial_int {
    ($type: ty) => {
impl NumberType for $type {
	#[cfg(feature = "text")]
	fn to_text(self) -> TextRepr {
		TextRepr::Integer(self as i64)
	}
	#[cfg(feature = "text")]
	fn from_i64(int: i64) -> Option<Self> {
		Some(int as $type)
	}
	#[cfg(feature = "text")]
	fn from_f64(_float: f64) -> Option<Self> {
		None
	}
	#[cfg(feature = "bin")]
	fn from_bin(bin:&mut VecDeque<u8>) -> Result<Self, DeserializationErrorKind> {
		Ok(Self::from_be_bytes(bin::split_first(bin)?))
	}
	#[cfg(feature = "bin")]
	fn to_bin(self) -> VecDeque<u8> {
		self.to_be_bytes().to_vec().into()
	}
}
impl_serde_number!($type);
	};
}

serial_int!(u8);
serial_int!(u16);
serial_int!(u32);
serial_int!(u64);
serial_int!(usize);
serial_int!(i8);
serial_int!(i16);
serial_int!(i32);
serial_int!(i64);
serial_int!(isize);


impl NumberType for f32 {
	#[cfg(feature = "text")]
	fn to_text(self) -> TextRepr {
		TextRepr::Float(self as f64)
	}

	#[cfg(feature = "bin")]
	fn from_bin(bin: &mut VecDeque<u8>) -> Result<Self, DeserializationErrorKind> {
		Ok(Self::from_be_bytes(bin::split_first(bin)?))
	}

	#[cfg(feature = "bin")]
	fn to_bin(self) -> VecDeque<u8> {
		self.to_be_bytes().into()
	}

	#[cfg(feature = "text")]
	fn from_i64(int: i64) -> Option<Self> {
		Some(int as Self)
	}

	#[cfg(feature = "text")]
	fn from_f64(float: f64) -> Option<Self> {
		Some(float as Self)
	}
}


impl NumberType for f64 {
	#[cfg(feature = "text")]
	fn to_text(self) -> TextRepr {
		TextRepr::Float(self)
	}

	#[cfg(feature = "bin")]
	fn from_bin(bin: &mut VecDeque<u8>) -> Result<Self, DeserializationErrorKind> {
		Ok(Self::from_be_bytes(bin::split_first(bin)?))
	}

	#[cfg(feature = "bin")]
	fn to_bin(self) -> VecDeque<u8> {
		self.to_be_bytes().into()
	}

	#[cfg(feature = "text")]
	fn from_i64(int: i64) -> Option<Self> {
		Some(int as Self)
	}

	#[cfg(feature = "text")]
	fn from_f64(float: f64) -> Option<Self> {
		Some(float)
	}
}

impl_serde_number!(f32);
impl_serde_number!(f64);

/// Implement Serialize for strings that can be converted to a String
macro_rules! serial_string {
    ($type: ty) => {
impl Serialize for $type {
	fn serialize<T: Serializer>(self, data: &mut T) {
		data.serialize_string(self);
	}
}
	};
}

serial_string!(String);
serial_string!(&str);

impl Deserialize for String {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		data.deserialize_string()
	}
}


/// Implement Deserialize for types that can be made from a String
macro_rules! from_string {
    ($type: ty) => {
impl Deserialize for $type {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		data.deserialize_string().and_then(|x| { Ok(<$type>::from(x)) })
	}
}
	};
}


from_string!(std::path::PathBuf);


impl Serialize for bool {
	fn serialize<T: Serializer>(self, data: &mut T) {
		data.serialize_bool(self);
	}
}


impl Deserialize for bool {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		data.deserialize_bool()
	}
}