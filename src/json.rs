use extern_json::JsonValue as Value;
use extern_json::object::Object;
use extern_json::{stringify_pretty, parse};

use super::*;

pub trait JSONSerde<ProfileMarker>: MappedSerde<ProfileMarker> {
	const TAB_SIZE: u16 = 4;
	fn serialize_json(self) -> String {
		stringify_pretty(self.serialize(Object::new()), Self::TAB_SIZE)
	}
	fn deserialize_json(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize(match parse(data.as_str())? {
			Value::Object(x) => x,
			_ => return Err(DeserializationError::from(
				DeserializationErrorKind::InvalidType { expected: "TomlObject", actual: "todo!" }
			))
		})
	}
}


impl MapAccess for Object {
	fn serialize_string(&mut self, name: &str, item: String) {
		self.insert(name.into(), Value::String(item));
	}

	fn deserialize_string(&mut self, name: &str) -> Result<String, DeserializationErrorKind> {
		match self.remove(name).ok_or(DeserializationErrorKind::MissingField)? {
			Value::String(x) => Ok(x),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "String", actual: "todo!" })
		}
	}

	fn serialize_u8(&mut self, name: &str, item: u8) {
		self.insert(name.into(), Value::from(item));
	}

	fn deserialize_u8(&mut self, name: &str) -> Result<u8, DeserializationErrorKind> {
		match self.remove(name).ok_or(DeserializationErrorKind::MissingField)? {
			Value::Number(x) => x.try_into().or(
				Err(DeserializationErrorKind::InvalidType { expected: "u8", actual: "todo!" })
			),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "String", actual: "todo!" })
		}
	}

	fn serialize_u16(&mut self, name: &str, item: u16) {
		self.insert(name.into(), Value::from(item));
	}

	fn deserialize_u16(&mut self, name: &str) -> Result<u16, DeserializationErrorKind> {
		match self.remove(name).ok_or(DeserializationErrorKind::MissingField)? {
			Value::Number(x) => x.try_into().or(
				Err(DeserializationErrorKind::InvalidType { expected: "u16", actual: "todo!" })
			),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "String", actual: "todo!" })
		}
	}
}
