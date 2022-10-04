use std::collections::VecDeque;
use std::str::FromStr;

use super::*;

pub mod prelude {
	pub use crate::{impl_bin, impl_bin_deser, impl_bin_ser};

	pub use super::{BinDeserialize, BinSerialize};
}


type Binary = VecDeque<u8>;


pub trait BinSerialize<P = NaturalProfile> {
	fn serialize_bin(self) -> Vec<u8>;
}


pub trait BinDeserialize<P = NaturalProfile>: Sized {
	fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into Binary with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait BinSerde: BinSerialize + BinDeserialize {}

impl<T: BinSerialize + BinDeserialize> BinSerde for T {}


pub trait MarshalledBinSerialize<Marshall> {
	fn serialize_bin(self, marshall: &Marshall) -> Vec<u8>;
}


pub trait MarshalledBinDeserialize<Marshall>: Sized {
	fn deserialize_bin(data: Vec<u8>, marshall: &Marshall) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into TOML with the same profile,
/// and the same type of marshall. Is automatically implemented on all appropriate types
pub trait MarshalledBinSerde<Marshall>: MarshalledBinSerialize<Marshall> + MarshalledBinDeserialize<Marshall> {}

impl<Marshall, T: MarshalledBinSerialize<Marshall> + MarshalledBinDeserialize<Marshall>> MarshalledBinSerde<Marshall> for T {}


#[macro_export]
macro_rules! impl_bin {
    ($name: ty, $profile: ty) => {
		impl_bin_ser!($name, $profile);
		impl_bin_deser!($name, $profile);
	};
    // ($name: ty, $profile: ty, $marshall: ty) => {
	// 	impl MarshalledBinSerde for $name {
	// 		fn serialize_bin(self, marshall: &$marshall) -> Vec<u8> {
	// 			let mut out = Vec::new();
	// 			MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
	// 			out
	// 		}
	// 		fn deserialize_bin(data: Vec<u8>, marshall: &$marshall) -> Result<Self, DeserializationError> {
	// 			MarshalledDeserialize::<$profile>::deserialize(&mut data, marshall)
	// 		}
	// 	}
	// };
}


#[macro_export]
macro_rules! impl_bin_ser {
    ($name: ty, $profile: ty) => {
		impl BinSerialize<$profile> for $name {
			fn serialize_bin(self) -> Vec<u8> {
				let mut out = std::collections::VecDeque::<u8>::new();
				Serialize::<$profile>::serialize(self, &mut out);
				out.into()
			}
		}
	};
    // ($name: ty, $profile: ty, $marshall: ty) => {
	// 	impl MarshalledBinSerde for $name {
	// 		fn serialize_bin(self, marshall: &$marshall) -> Vec<u8> {
	// 			let mut out = Vec::new();
	// 			MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
	// 			out
	// 		}
	// 		fn deserialize_bin(data: Vec<u8>, marshall: &$marshall) -> Result<Self, DeserializationError> {
	// 			MarshalledDeserialize::<$profile>::deserialize(&mut data, marshall)
	// 		}
	// 	}
	// };
}


#[macro_export]
macro_rules! impl_bin_deser {
    ($name: ty, $profile: ty) => {
		impl BinDeserialize<$profile> for $name {
			fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize(&mut Into::<std::collections::VecDeque<u8>>::into(data))
			}
		}
	};
    // ($name: ty, $profile: ty, $marshall: ty) => {
	// 	impl MarshalledBinSerde for $name {
	// 		fn serialize_bin(self, marshall: &$marshall) -> Vec<u8> {
	// 			let mut out = Vec::new();
	// 			MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
	// 			out
	// 		}
	// 		fn deserialize_bin(data: Vec<u8>, marshall: &$marshall) -> Result<Self, DeserializationError> {
	// 			MarshalledDeserialize::<$profile>::deserialize(&mut data, marshall)
	// 		}
	// 	}
	// };
}


pub(crate) fn split_first<const SIZE: usize>(bytes: &mut Binary) -> Result<[u8; SIZE], DeserializationErrorKind> {
	if bytes.len() < SIZE {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	let out: Vec<u8> = bytes.drain(0..SIZE).collect();
	Ok(out.try_into().unwrap())
}


pub(crate) fn split_first_vec(bytes: &mut Binary, size: usize) -> Result<Binary, DeserializationErrorKind> {
	if bytes.len() < size {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	Ok(bytes.drain(0..size).collect())
}


fn size_to_bytes(size: usize, size_type: SizeType) -> Binary {
	match size_type {
		SizeType::U8 => vec![size as u8].into(),
		SizeType::U16 => (size as u16).to_bin(),
		SizeType::U32 => (size as u32).to_bin(),
	}
}


fn bytes_to_size(bytes: &mut Binary, size_type: SizeType) -> Result<usize, DeserializationErrorKind> {
	match size_type {
		SizeType::U8 => Ok(bytes.pop_front().ok_or(DeserializationErrorKind::UnexpectedEOF)? as usize),
		SizeType::U16 => Ok(u16::from_bin(bytes)? as usize),
		SizeType::U32 => Ok(u32::from_bin(bytes)? as usize),
	}
}


/// Iterates through the given bytes vec to find a window with the given key.
/// Returns the index after the end of the window
fn find_key<K: Borrow<str>>(bytes: &[u8], key: K) -> Option<usize> {
	let key_bytes = key.borrow().as_bytes();
	let bytes_len = key_bytes.len();
	for (i, window) in bytes.windows(bytes_len).enumerate() {
		if window == key_bytes {
			return Some(i + bytes_len)
		}
	}
	None
}


// trait MissingField {
// 	fn missing() -> Self;
// }
//
//
// impl MissingField for DeserializationErrorKind {
// 	fn missing() -> Self {
// 		DeserializationErrorKind::MissingField
// 	}
// }
//
//
// impl MissingField for DeserializationError {
// 	fn missing() -> Self {
// 		Self::from(DeserializationErrorKind::missing())
// 	}
// }

/// Deserialize a type, using the given fn, at the given key
fn key_deserialize<T, F>(bytes: &mut Binary, key: &str, f: F) -> Result<T, DeserializationError>
	where
		F: Fn(&mut Binary) -> Result<T, DeserializationError>
{
	let key = key.to_string();
	let idx = find_key(bytes.make_contiguous(), key.clone()).ok_or_else(|| DeserializationError::missing_field(key.clone()))?;
	let mut last = bytes.drain(idx..).collect();
	let item = (f)(&mut last).map_err(|e| { DeserializationError::nest(e).set_field(key) })?;
	bytes.append(&mut last);
	Ok(item)
}

impl PrimitiveSerializer for Binary {
	fn serialize_bool(&mut self, boolean: bool) {
		self.push_back(255 * boolean as u8);
	}

	fn deserialize_bool(&mut self) -> Result<bool, DeserializationError> {
		if self.is_empty() {
			return Err(DeserializationError::new_kind(DeserializationErrorKind::UnexpectedEOF))
		}
		match self.pop_front().ok_or_else(|| DeserializationError::new_kind(DeserializationErrorKind::UnexpectedEOF))? {
			255 => Ok(true),
			0 => Ok(false),
			x => Err(DeserializationError::new_kind(DeserializationErrorKind::NoMatch { actual: x.to_string() }))
		}
	}

	fn serialize_num<T: NumberType>(&mut self, num: T) {
		self.append(&mut num.to_bin())
	}

	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError> {
		T::from_bin(self).map_err(DeserializationError::new_kind)
	}

	fn serialize_string<T: Into<String>>(&mut self, string: T) {
		let mut bytes: VecDeque<u8> = string.into().as_bytes().to_vec().into();
		self.append(&mut size_to_bytes(bytes.len(), SizeType::U32));
		self.append(&mut bytes);
	}

	fn deserialize_string(&mut self) -> Result<String, DeserializationError> {
		let size = bytes_to_size(self, SizeType::U32).no_field()?;
		String::from_utf8(split_first_vec(self, size).no_field()?.into()).map_err(|e| { DeserializationError::new_kind(DeserializationErrorKind::FromUTF8Error(e)) })
	}

	fn serialize_bytes<T: Into<VecDeque<u8>>>(&mut self, bytes: T) {
		let mut bytes = bytes.into();
		self.serialize_num(bytes.len() as u32);
		self.append(&mut bytes);
	}

	fn deserialize_bytes<T: FromIterator<u8>>(&mut self) -> Result<T, DeserializationError> {
		let size = self.deserialize_num::<u32>()? as usize;
		Ok(self.drain(0..size).collect())
	}
}


impl Serializer for Binary {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T) {
		item.serialize(self);
	}

	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T) {
		self.append(&mut key.borrow().to_string().as_bytes().to_vec().into());
		item.serialize(self);
	}

	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError> {
		T::deserialize::<Self>(self)
	}

	fn deserialize_key_internal<P, T: Deserialize<P>>(&mut self, key: &str) -> Result<T, DeserializationError> {
		key_deserialize(self, key, |x| { T::deserialize::<Self>(x) })
	}

	fn try_get_key<K: FromStr>(&mut self) -> Option<K> {
		self.deserialize_string().ok().map(|x| K::from_str(x.as_str()).ok()).flatten()
	}
}
