use crate::array::{SizeType};
use super::*;


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


macro_rules! size_to_bytes {
    ($size_type: expr, $size: expr) => {
		match $size_type {
			SizeType::U8 => ($size as u8).to_be_bytes().to_vec(),
			SizeType::U16 => ($size as u16).to_be_bytes().to_vec(),
			SizeType::U32 => ($size as u32).to_be_bytes().to_vec(),
		}
	};
}


macro_rules! bytes_to_size {
    ($size_type: expr, $bytes: expr) => {
		match $size_type {
			SizeType::U8 => if $bytes.is_empty() {
								Err(DeserializationErrorKind::UnexpectedEOF)
							} else {
								Ok($bytes.remove(0) as usize)
							}
			SizeType::U16 => Ok(u16::from_be_bytes(split_first($bytes)?) as usize),
			SizeType::U32 => Ok(u32::from_be_bytes(split_first($bytes)?) as usize),
		}
	};
}


pub trait BinSerde<ProfileMarker>: ArraySerde<ProfileMarker> {
	const DEFAULT_CAPACITY: usize = 16;

	fn serialize_bin(self) -> Vec<u8> {
		self.serialize(Vec::with_capacity(Self::DEFAULT_CAPACITY))
	}
	fn deserialize_bin(data: Vec<u8>) -> Result<Self, DeserializationError> {
		Self::deserialize(data)
	}
}


impl ArrayAccess for Vec<u8> {
	fn serialize_string(&mut self, string: String, size_type: SizeType) {
		let mut bytes = string.as_bytes().to_vec();
		self.append(&mut size_to_bytes!(size_type, bytes.len()));
		self.append(&mut bytes);
	}

	fn serialize_string_unsized(&mut self, string: String) {
		self.append(&mut string.as_bytes().to_vec());
	}

	fn deserialize_string(&mut self, size_type: SizeType) -> Result<String, DeserializationErrorKind> {
		let size = bytes_to_size!(size_type, self)?;
		String::from_utf8(
			split_first_vec(self, size)?
		).map_err(Into::into)
	}

	fn deserialize_string_sized(&mut self, size: usize) -> Result<String, DeserializationErrorKind> {
		String::from_utf8(split_first_vec(self, size)?).map_err(Into::into)
	}

	fn serialize_u8(&mut self, num: u8) {
		self.push(num);
	}

	fn deserialize_u8(&mut self) -> Result<u8, DeserializationErrorKind> {
		if self.is_empty() {
			return Err(DeserializationErrorKind::UnexpectedEOF)
		}
		Ok(self.remove(0))
	}

	fn serialize_u16(&mut self, num: u16) {
		self.append(&mut num.to_be_bytes().to_vec());
	}

	fn deserialize_u16(&mut self) -> Result<u16, DeserializationErrorKind> {
		Ok(u16::from_be_bytes(split_first(self)?))
	}
}
