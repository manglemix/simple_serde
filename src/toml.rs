use std::collections::{HashMap};
use std::hash::Hash;
use std::mem::replace;
use extern_toml::Value;
use extern_toml::value::{Array, Table};

use super::*;


pub trait TOMLSerde: Sized {
	fn serialize_toml(self) -> String;
	fn deserialize_toml(data: String) -> Result<Self, DeserializationError>;
}


pub trait MarshalledTOMLSerde<Marshall>: Sized {
	fn serialize_toml(self, marshall: &Marshall) -> String;
	fn deserialize_toml(data: String, marshall: &Marshall) -> Result<Self, DeserializationError>;
}


#[macro_export]
macro_rules! impl_toml {
    ($name: ty, $profile: ty) => {
		impl crate::toml::TOMLSerde for $name {
			fn serialize_toml(self) -> String {
				Serialize::<$profile>::serialize::<extern_toml::Value>(self).to_string()
			}
			fn deserialize_toml(data: String) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize::<extern_toml::Value, extern_toml::Value>(data.parse()?)
			}
		}
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl crate::toml::MarshalledTOMLSerde for $name {
			fn serialize_toml(self, marshall: &$marshall) -> String {
				MarshalledSerialize::<$profile>::serialize::<extern_toml::Value>(self, marshall).to_string()
			}
			fn deserialize_toml(data: String, marshall: &$marshall) -> Result<Self, DeserializationError> {
				MarshalledDeserialize::<$profile>::deserialize::<extern_toml::Value>(data.parse()?, marshall)
			}
		}
	};
}

pub use impl_toml;


fn push_value(origin: &mut Value, item: Value) {
	match origin {
		Value::Array(x) => x.push(item),
		Value::Table(x) => 	if x.is_empty() {
											*origin = item;
										} else {
											panic!("Tried to push onto table!")
										}
		_ => {
			let last = replace(origin, Value::Array(Array::new()));
			let arr = match origin {
				Value::Array(x) => x,
				_ => unreachable!()
			};
			arr.push(last);
			arr.push(item);
		}
	}
}


fn push_entry<K: Into<String>>(origin: &mut Value, key: K, item: Value) {
	match origin {
		Value::Table(x) => { x.insert(key.into(), item); }
		Value::Array(x) => {
			let mut table = Table::new();
			table.insert(key.into(), item);
			x.push(Value::Table(table));
		}
		_ => {
			let last = replace(origin, Value::Array(Array::new()));
			let arr = match origin {
				Value::Array(x) => x,
				_ => unreachable!()
			};
			arr.push(last);

			let mut table = Table::new();
			table.insert(key.into(), item);

			arr.push(Value::Table(table));
		}
	}
}


impl PrimitiveSerializer for Value {
	fn serialize_num<T: NumberType>(&mut self, num: T) {
		let val = match num.to_simple() {
			SimpleNumber::I64(x) => Self::Integer(x),
			SimpleNumber::F64(x) => Self::Float(x)
		};
		push_value(self, val);
	}

	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError> {
		match self {
			Value::Integer(x) => T::from_i64(x.clone()).ok_or(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "unsigned int", actual: "signed int" })),
			Value::Float(x) => T::from_f64(x.clone()).ok_or(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "integer", actual: "float" })),
			Value::Array(x) => x.remove(0).deserialize_num(),
			_ => Err(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "number", actual: "todo!" }))
		}
	}

	fn serialize_string<T: Into<String>>(&mut self, string: T) {
		push_value(self, Value::String(string.into()));
	}

	fn deserialize_string(&mut self) -> Result<String, DeserializationError> {
		match self {
			Value::String(x) => Ok(x.clone()),
			Value::Array(x) => x.remove(0).deserialize_string(),
			_ => Err(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "string", actual: "todo!" }))
		}
	}
}

impl Serializer for Value {
	fn empty() -> Self {
		Self::Table(Table::new())
	}

	fn serialize<P, T: Serialize<P>>(&mut self, item: T) {
		push_value(self, item.serialize());
	}

	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T) {
		push_entry(self, key.borrow(), item.serialize());
	}

	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError> {
		match self {
			Self::Array(x) => T::deserialize(x.remove(0)),
			_ => T::deserialize(self.clone())
		}
	}

	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError> {
		match self {
			Self::Table(x) => T::deserialize(x.remove(key.borrow()).ok_or(DeserializationError::from(DeserializationErrorKind::MissingField))?),
			_ => Err(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "table", actual: "todo!" }))
		}
	}
}


impl<'a, K, V> TOMLSerde for HashMap<K, V>
	where
		K: Hash + Eq + extern_toml::macros::Deserialize<'a>,
		V: extern_toml::macros::Deserialize<'a>,
		String: From<K>,
		Value: From<V>
{
	fn serialize_toml(self) -> String {
		let val: Value = From::<HashMap<K, V>>::from(self);
		val.to_string()
	}

	fn deserialize_toml(data: String) -> Result<Self, DeserializationError> {
		let value: Value = data.parse()?;
		match &value {
			Value::Table(_) => {
				value.try_into().map_err(Into::into)
			}
			_ => Err(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "Table", actual: "todo!" }))
		}
	}
}
