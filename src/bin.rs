use super::*;


pub trait BinSerde: Sized {
	fn serialize_bin(self) -> Vec<u8>;
	fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError>;
}


#[macro_export]
macro_rules! impl_bin {
    ($name: ty, $profile: ty) => {
		impl crate::bin::BinSerde for $name {
			fn serialize_bin(self) -> Vec<u8> {
				Serialize::<$profile>::serialize(self)
			}
			fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize(data)
			}
		}
	};
}


fn split_first<const SIZE: usize>(bytes: &mut Vec<u8>) -> Result<[u8; SIZE], DeserializationErrorKind> {
	if bytes.len() < SIZE {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	let (first, last) = bytes.split_at(SIZE);
	let out = first.try_into().unwrap();
	*bytes = last.to_vec();
	Ok(out)
}


fn split_first_vec(bytes: &mut Vec<u8>, size: usize) -> Result<Vec<u8>, DeserializationErrorKind> {
	if bytes.len() < size {
		return Err(DeserializationErrorKind::UnexpectedEOF)
	}
	let (first, last) = bytes.split_at(size);
	let out = first.to_vec();
	*bytes = last.to_vec();
	Ok(out)
}


trait ToBytesVec: Sized {
	fn to_bytes_vec(self) -> Vec<u8>;
	fn append_bytes(self, bytes: &mut Vec<u8>) {
		bytes.append(&mut self.to_bytes_vec());
	}
}


trait FromBytesVecSized: Sized {
	fn from_bytes_vec_sized(bytes: &mut Vec<u8>) -> Result<Self, DeserializationErrorKind>;
}


trait FromBytesVec: Sized {
	fn from_bytes_vec(bytes: Vec<u8>) -> Result<Self, DeserializationErrorKind>;
}


/// Implements Bytes Serialization for number types
macro_rules! impl_num_bytes {
    ($type: ty) => {
		impl ToBytesVec for $type {
			fn to_bytes_vec(self) -> Vec<u8> {
				self.to_be_bytes().to_vec()
			}
		}
		impl FromBytesVecSized for $type {
			fn from_bytes_vec_sized(bytes: &mut Vec<u8>) -> Result<Self, DeserializationErrorKind> {
				Ok(Self::from_be_bytes(split_first(bytes)?))
			}
		}
	};
}

impl_num_bytes!(u16);
impl_num_bytes!(u32);


impl ToBytesVec for String {
	fn to_bytes_vec(self) -> Vec<u8> {
		self.as_bytes().to_vec()
	}
}
impl ToBytesVec for &str {
	fn to_bytes_vec(self) -> Vec<u8> {
		self.as_bytes().to_vec()
	}
}
impl FromBytesVec for String {
	fn from_bytes_vec(bytes: Vec<u8>) -> Result<Self, DeserializationErrorKind> {
		Self::from_utf8(bytes).map_err(Into::into)
	}
}


fn size_to_bytes(size: usize, size_type: SizeType) -> Vec<u8> {
	match size_type {
		SizeType::U8 => vec![size as u8],
		SizeType::U16 => (size as u16).to_bytes_vec(),
		SizeType::U32 => (size as u32).to_bytes_vec(),
	}
}


fn bytes_to_size(bytes: &mut Vec<u8>, size_type: SizeType) -> Result<usize, DeserializationErrorKind> {
	match size_type {
		SizeType::U8 => if bytes.is_empty() {
							Err(DeserializationErrorKind::UnexpectedEOF)
						} else {
							Ok(bytes.remove(0) as usize)
						}
		SizeType::U16 => Ok(u16::from_bytes_vec_sized(bytes)? as usize),
		SizeType::U32 => Ok(u32::from_bytes_vec_sized(bytes)? as usize),
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


impl SerializeItem<u8> for Vec<u8> {
	fn serialize(&mut self, item: u8) {
		self.push(item);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: u8) {
		key.borrow().append_bytes(self);
		self.push(item);
	}
}

// /// Implement SerializeItem for all types that implement ToBytesVec
// impl<T: ToBytesVec> SerializeItem<T> for Vec<u8> {
// 	fn serialize(&mut self, item: T) {
// 		item.append_bytes(self);
// 	}
//
// 	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: T) {
// 		key.borrow().append_bytes(self);
// 		item.append_bytes(self);
// 	}
// }

macro_rules! impl_to_bytes {
    ($type: ty) => {
impl SerializeItem<$type> for Vec<u8> {
	fn serialize(&mut self, item: $type) {
		item.append_bytes(self);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: $type) {
		key.borrow().append_bytes(self);
		item.append_bytes(self);
	}
}
	};
}


// impl_to_bytes!(u64);
// impl_to_bytes!(u32);
impl_to_bytes!(u16);
impl_to_bytes!(String);

/// Deserialize a type, using the given fn, at the given key
fn key_deserialize<T, K, F>(bytes: &mut Vec<u8>, key: K, f: F) -> Result<T, DeserializationErrorKind>
	where
		K: Borrow<str>,
		F: Fn(&mut Vec<u8>) -> Result<T, DeserializationErrorKind>
{
	let idx = find_key(bytes, key).ok_or(DeserializationErrorKind::MissingField)?;
	let (first, last) = bytes.split_at(idx);
	let mut last = last.to_vec();
	*bytes = first.to_vec();
	let item = (f)(&mut last)?;
	bytes.append(&mut last);
	Ok(item)
}


/// Implement DeserializeItem for all types that implement FromBytesVecSized
impl<T: FromBytesVecSized> DeserializeItem<T> for Vec<u8> {
	fn deserialize(&mut self) -> Result<T, DeserializationErrorKind> {
		T::from_bytes_vec_sized(self)
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationErrorKind> {
		key_deserialize(self, key, T::from_bytes_vec_sized)
	}
}


/// Implement SerializeItemAutoSize for all types that can implement ToBytesVec
impl<T: ToBytesVec> SerializeItemAutoSize<T> for Vec<u8> {
	fn serialize(&mut self, item: T, size_type: SizeType) {
		let mut bytes = item.to_bytes_vec();
		self.append(&mut size_to_bytes(bytes.len(), size_type));
		self.append(&mut bytes);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: T, size_type: SizeType) {
		key.borrow().append_bytes(self);
		SerializeItemAutoSize::serialize(self, item, size_type);
	}
}

impl DeserializeItem<u8> for Vec<u8> {
	fn deserialize(&mut self) -> Result<u8, DeserializationErrorKind> {
		if self.is_empty() {
			Err(DeserializationErrorKind::UnexpectedEOF)
		} else {
			Ok(self.remove(0))
		}
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<u8, DeserializationErrorKind> {
		let idx = find_key(self, key).ok_or(DeserializationErrorKind::MissingField)?;
		Ok(self.remove(idx))
	}
}


/// Implement DeserializeItemVarSize for all types that implement FromBytesVec
impl<T: FromBytesVec> DeserializeItemVarSize<T> for Vec<u8> {
	fn deserialize(&mut self, size: usize) -> Result<T, DeserializationErrorKind> {
		T::from_bytes_vec(split_first_vec(self, size)?)
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, size: usize) -> Result<T, DeserializationErrorKind> {
		key_deserialize(self, key, |x| { DeserializeItemVarSize::deserialize(x, size) })
	}
}


/// Implement DeserializeItemAutoSize for all types that implement DeserializeItemVarSize
impl<T> DeserializeItemAutoSize<T> for Vec<u8> where Vec<u8>: DeserializeItemVarSize<T> {
	fn deserialize(&mut self, size_type: SizeType) -> Result<T, DeserializationErrorKind> {
		let size = bytes_to_size(self, size_type)?;
		DeserializeItemVarSize::deserialize(self, size)
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, size_type: SizeType) -> Result<T, DeserializationErrorKind> {
		key_deserialize(self, key, move |x| { DeserializeItemAutoSize::deserialize(x, size_type) })
	}
}

impl ItemAccess for Vec<u8> {
	fn empty() -> Self {
		Self::new()
	}
}
