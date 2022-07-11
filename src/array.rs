use super::*;

pub enum SizeType {
	U8,
	U16,
	U32
}


pub trait ArrayAccess {
	fn serialize_string(&mut self, string: String, size_type: SizeType);
	fn serialize_string_unsized(&mut self, string: String);
	fn deserialize_string(&mut self, size_type: SizeType) -> Result<String, DeserializationErrorKind>;
	fn deserialize_string_sized(&mut self, size: usize) -> Result<String, DeserializationErrorKind>;
	fn serialize_u8(&mut self, num: u8);
	fn deserialize_u8(&mut self) -> Result<u8, DeserializationErrorKind>;
	fn serialize_u16(&mut self, num: u16);
	fn deserialize_u16(&mut self) -> Result<u16, DeserializationErrorKind>;
}


pub struct ArrayDataContainer<T: ArrayAccess> {
	pub(crate) serializing: bool,
	pub(crate) data: T
}


impl<T: ArrayAccess> ArrayDataContainer<T> {
	pub fn is_serializing(&self) -> bool {
		self.serializing
	}
}


pub trait SerdeArrayItemSized<T> {
	fn serialize(&mut self, item: T);
	fn deserialize(&mut self) -> Result<T, DeserializationError>;
	fn serde(&mut self, into: &mut T) -> Result<(), DeserializationError>;
}


pub trait SerdeArrayItemUnsized<T> {
	fn serialize(&mut self, item: T, size_type: SizeType);
	fn serialize_unsized(&mut self, item: T);
	fn deserialize(&mut self, size_type: SizeType) -> Result<T, DeserializationError>;
	fn deserialize_sized(&mut self, size: usize) -> Result<T, DeserializationError>;
	fn serde(&mut self, into: &mut T, size_type: SizeType) -> Result<(), DeserializationError>;
	fn serde_sized(&mut self, into: &mut T, size: usize) -> Result<(), DeserializationError>;
}


macro_rules! impl_serde_unsized {
    ($T: ty) => {
		fn serde(&mut self, into: &mut $T, size_type: SizeType) -> Result<(), DeserializationError> {
            if self.serializing {
				SerdeArrayItemUnsized::serialize(self, std::mem::take(into), size_type);
            } else {
				*into = SerdeArrayItemUnsized::deserialize(self, size_type)?;
            }
            Ok(())
        }
		fn serde_sized(&mut self, into: &mut $T, size: usize) -> Result<(), DeserializationError> {
            if self.serializing {
				SerdeArrayItemUnsized::serialize_unsized(self, std::mem::take(into));
            } else {
				*into = SerdeArrayItemUnsized::deserialize_sized(self, size)?;
            }
            Ok(())
        }
    };
}
macro_rules! impl_serde {
    ($T: ty) => {
		fn serde(&mut self, into: &mut $T) -> Result<(), DeserializationError> {
			if self.serializing {
				SerdeArrayItemSized::serialize(self, std::mem::take(into));
			} else {
				*into = SerdeArrayItemSized::deserialize(self)?;
			}
			Ok(())
		}
	};
}


impl<T: ArrayAccess> SerdeArrayItemUnsized<String> for ArrayDataContainer<T> {
	fn serialize(&mut self, item: String, size_type: SizeType) {
		self.data.serialize_string(item, size_type);
	}

	fn serialize_unsized(&mut self, item: String) {
		self.data.serialize_string_unsized(item);
	}

	fn deserialize(&mut self, size_type: SizeType) -> Result<String, DeserializationError> {
		self.data.deserialize_string(size_type).map_err(
			DeserializationError::from
		)
	}

	fn deserialize_sized(&mut self, size: usize) -> Result<String, DeserializationError> {
		self.data.deserialize_string_sized(size).map_err(
			DeserializationError::from
		)
	}
	impl_serde_unsized!(String);
}


impl<T: ArrayAccess> SerdeArrayItemSized<u8> for ArrayDataContainer<T> {
	fn serialize(&mut self, item: u8) {
		self.data.serialize_u8(item);
	}

	fn deserialize(&mut self) -> Result<u8, DeserializationError> {
		self.data.deserialize_u8().map_err(
			DeserializationError::from
		)
	}
	impl_serde!(u8);
}


impl<T: ArrayAccess> SerdeArrayItemSized<u16> for ArrayDataContainer<T> {
	fn serialize(&mut self, item: u16) {
		self.data.serialize_u16(item);
	}

	fn deserialize(&mut self) -> Result<u16, DeserializationError> {
		self.data.deserialize_u16().map_err(
			DeserializationError::from
		)
	}
	impl_serde!(u16);
}