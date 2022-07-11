use super::*;

pub trait MapAccess {
	fn serialize_string(&mut self, name: &str, item: String);
	fn deserialize_string(&mut self, name: &str) -> Result<String, DeserializationErrorKind>;
	fn serialize_u8(&mut self, name: &str, item: u8);
	fn deserialize_u8(&mut self, name: &str) -> Result<u8, DeserializationErrorKind>;
	fn serialize_u16(&mut self, name: &str, item: u16);
	fn deserialize_u16(&mut self, name: &str) -> Result<u16, DeserializationErrorKind>;
}


pub struct MapDataContainer<T: MapAccess> {
	pub(crate) serializing: bool,
	pub(crate) data: T
}


impl<T: MapAccess> MapDataContainer<T> {
	pub fn is_serializing(&self) -> bool {
		self.serializing
	}
}


macro_rules! impl_serde {
    ($T: ty) => {
        fn serde(&mut self, name: &str, into: &mut $T) -> Result<(), DeserializationError> {
            if self.serializing {
                self.serialize(name, std::mem::take(into));
            } else {
                *into = self.deserialize(name)?;
            }
            Ok(())
        }
    };
}


pub trait SerdeMapItem<T> {
	fn serialize(&mut self, name: &str, item: T);
	fn deserialize(&mut self, name: &str) -> Result<T, DeserializationError>;
	fn serde(&mut self, name: &str, into: &mut T) -> Result<(), DeserializationError>;
}


impl<T: MapAccess> SerdeMapItem<String> for MapDataContainer<T> {
	fn serialize(&mut self, name: &str, item: String) {
		self.data.serialize_string(name, item);
	}

	fn deserialize(&mut self, name: &str) -> Result<String, DeserializationError> {
		self.data.deserialize_string(name).map_err(|e| {
			DeserializationError::new(name, e)
		})
	}
	impl_serde!(String);
}


impl<T: MapAccess> SerdeMapItem<u8> for MapDataContainer<T> {
	fn serialize(&mut self, name: &str, item: u8) {
		self.data.serialize_u8(name, item);
	}

	fn deserialize(&mut self, name: &str) -> Result<u8, DeserializationError> {
		self.data.deserialize_u8(name).map_err(|e| {
			DeserializationError::new(name, e)
		})
	}
	impl_serde!(u8);
}


impl<T: MapAccess> SerdeMapItem<u16> for MapDataContainer<T> {
	fn serialize(&mut self, name: &str, item: u16) {
		self.data.serialize_u16(name, item);
	}

	fn deserialize(&mut self, name: &str) -> Result<u16, DeserializationError> {
		self.data.deserialize_u16(name).map_err(|e| {
			DeserializationError::new(name, e)
		})
	}
	impl_serde!(u16);
}
