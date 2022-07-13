use super::*;


pub trait BinSerde: Sized {
	fn serialize_bin(self) -> Vec<u8>;
	fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError>;
}


pub trait MarshalledBinSerde<Marshall>: Sized {
	fn serialize_bin(self, marshall: &Marshall) -> Vec<u8>;
	fn deserialize_bin(data: Vec<u8>, marshall: &Marshall) -> Result<Self, DeserializationError>;
}


#[macro_export]
macro_rules! impl_bin {
    ($name: ty, $profile: ty) => {
		impl crate::bin::BinSerde for $name {
			fn serialize_bin(self) -> Vec<u8> {
				Serialize::<$profile>::serialize(self)
			}
			fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize::<Vec<u8>, _>(data)
			}
		}
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl crate::toml::MarshalledBinSerde for $name {
			fn serialize_bin(self, marshall: &$marshall) -> Vec<u8> {
				MarshalledSerialize::<$profile>::serialize(self, marshall)
			}
			fn deserialize_bin(data: Vec<u8>, marshall: &$marshall) -> Result<Self, DeserializationError> {
				MarshalledDeserialize::<$profile>::deserialize(data, marshall)
			}
		}
	};
}


pub use impl_bin;


pub(crate) fn split_first<const SIZE: usize>(bytes: &mut Vec<u8>) -> Result<[u8; SIZE], DeserializationErrorKind> {
	if bytes.len() < SIZE {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	let (first, last) = bytes.split_at(SIZE);
	let out = first.try_into().unwrap();
	*bytes = last.to_vec();
	Ok(out)
}


pub(crate) fn split_first_vec(bytes: &mut Vec<u8>, size: usize) -> Result<Vec<u8>, DeserializationErrorKind> {
	if bytes.len() < size {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	let (first, last) = bytes.split_at(size);
	let out = first.to_vec();
	*bytes = last.to_vec();
	Ok(out)
}


fn size_to_bytes(size: usize, size_type: SizeType) -> Vec<u8> {
	match size_type {
		SizeType::U8 => vec![size as u8],
		SizeType::U16 => (size as u16).to_bin(),
		SizeType::U32 => (size as u32).to_bin(),
	}
}


fn bytes_to_size(bytes: &mut Vec<u8>, size_type: SizeType) -> Result<usize, DeserializationErrorKind> {
	match size_type {
		SizeType::U8 => if bytes.is_empty() {
							Err(DeserializationErrorKind::UnexpectedEOF)
						} else {
							Ok(bytes.remove(0) as usize)
						}
		SizeType::U16 => Ok(u16::from_bin(bytes)? as usize),
		SizeType::U32 => Ok(u32::from_bin(bytes)? as usize),
	}
}


/// Iterates through the given bytes vec to find a window with the given key.
/// Returns the index after the end of the window
fn find_key<K: Borrow<str>>(bytes: &Vec<u8>, key: K) -> Option<usize> {
	let key_bytes = key.borrow().as_bytes();
	let bytes_len = key_bytes.len();
	let mut i = 0usize;
	for window in bytes.windows(bytes_len) {
		if window == key_bytes {
			return Some(i + bytes_len)
		}
		i += 1;
	}
	None
}


trait MissingField {
	fn missing() -> Self;
}


impl MissingField for DeserializationErrorKind {
	fn missing() -> Self {
		DeserializationErrorKind::MissingField
	}
}


impl MissingField for DeserializationError {
	fn missing() -> Self {
		Self::from(DeserializationErrorKind::missing())
	}
}

/// Deserialize a type, using the given fn, at the given key
fn key_deserialize<T, K, F, E>(bytes: &mut Vec<u8>, key: K, f: F) -> Result<T, E>
	where
		K: Borrow<str>,
		F: Fn(&mut Vec<u8>) -> Result<T, E>,
		E: MissingField
{
	let idx = find_key(bytes, key).ok_or(E::missing())?;
	let (first, last) = bytes.split_at(idx);
	let mut last = last.to_vec();
	*bytes = first.to_vec();
	let item = (f)(&mut last)?;
	bytes.append(&mut last);
	Ok(item)
}


impl PrimitiveSerializer for Vec<u8> {
	fn serialize_num<T: NumberType>(&mut self, num: T) {
		self.append(&mut num.to_bin())
	}

	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError> {
		T::from_bin(self).map_err(Into::into)
	}

	fn serialize_string<T: Into<String>>(&mut self, string: T) {
		let mut bytes = string.into().as_bytes().to_vec();
		self.append(&mut size_to_bytes(bytes.len(), SizeType::U32));
		self.append(&mut bytes);
	}

	fn deserialize_string(&mut self) -> Result<String, DeserializationError> {
		let size = bytes_to_size(self, SizeType::U32)?;
		String::from_utf8(split_first_vec(self, size)?).map_err(Into::into)
	}
}

impl Serializer for Vec<u8> {
	fn empty() -> Self {
		Self::new()
	}

	fn serialize<P, T: Serialize<P>>(&mut self, item: T) {
		self.append(&mut item.serialize());
	}

	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T) {
		self.append(&mut key.borrow().as_bytes().to_vec());
		self.append(&mut item.serialize());
	}

	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError> {
		T::deserialize::<Self, _>(self)
	}

	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError> {
		key_deserialize(self, key, |x| { T::deserialize::<Self, _>(x) })
	}
}
