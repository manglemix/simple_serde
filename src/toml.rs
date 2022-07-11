use extern_toml::Value;
use extern_toml::value::Table;

use super::*;

pub trait TOMLSerde<ProfileMarker>: MappedSerde<ProfileMarker> {
	fn serialize_toml(self) -> String {
		Value::Table(self.serialize(Table::new())).to_string()
	}
	fn deserialize_toml(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize(match data.parse::<Value>()? {
			Value::Table(x) => x,
			_ => return Err(DeserializationError::from(
				DeserializationErrorKind::InvalidType { expected: "TomlTable", actual: "todo!" }
			))
		})
	}
}


impl MapAccess for Table {
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
		self.insert(name.into(), Value::Integer(item as i64));
	}

	fn deserialize_u8(&mut self, name: &str) -> Result<u8, DeserializationErrorKind> {
		match self.remove(name).ok_or(DeserializationErrorKind::MissingField)? {
			Value::Integer(x) => x.try_into().or(
				Err(DeserializationErrorKind::InvalidType { expected: "u8", actual: "todo!" })
			),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "String", actual: "todo!" })
		}
	}

	fn serialize_u16(&mut self, name: &str, item: u16) {
		self.insert(name.into(), Value::Integer(item as i64));
	}

	fn deserialize_u16(&mut self, name: &str) -> Result<u16, DeserializationErrorKind> {
		match self.remove(name).ok_or(DeserializationErrorKind::MissingField)? {
			Value::Integer(x) => x.try_into().or(
				Err(DeserializationErrorKind::InvalidType { expected: "u16", actual: "todo!" })
			),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "String", actual: "todo!" })
		}
	}
}
