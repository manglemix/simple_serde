use std::collections::HashMap;
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
				Deserialize::<$profile>::deserialize::<extern_toml::Value>(data.parse()?)
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


macro_rules! impl_ser_from {
    ($type: ty) => {
impl SerializeItem<$type> for Value {
	fn serialize(&mut self, item: $type) {
		match self {
			Self::Array(arr) => { arr.push(Value::from(item)); },
			_ => {
				let value = replace(self, Value::Array(Array::new()));
				let arr = match self {
					Self::Array(x) => x,
					_ => unreachable!()
				};
				arr.push(value);
				arr.push(Value::from(item));
			}
		}
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: $type) {
		match self {
			Self::Table(x) => { x.insert(key.borrow().into(), Value::from(item)); },
			Self::Array(x) => {
				if let Some(Self::Table(x)) = x.last_mut() {
					x.insert(key.borrow().into(), Value::from(item));
					return
				}
				let mut table = Table::new();
				table.insert(key.borrow().into(), Value::from(item));
				x.push(Value::Table(table));
			}
			_ => {
				let value = replace(self, Value::Array(Array::new()));
				let arr = match self {
					Self::Array(x) => x,
					_ => unreachable!()
				};
				arr.push(value);
				let mut table = Table::new();
				table.insert(key.borrow().into(), Value::from(item));
				arr.push(Value::Table(table));
			}
		};
	}
}
	};
}


impl_ser_from!(String);
impl_ser_from!(u32);
impl_ser_from!(u8);


impl SerializeItem<u16> for Value {
	fn serialize(&mut self, item: u16) {
		SerializeItem::serialize(self, item as u32);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: u16) {
		SerializeItem::serialize_key(self, key, item as u32);
	}
}


impl DeserializeItem<i64> for Value {
	fn deserialize(&mut self) -> Result<i64, DeserializationErrorKind> {
		match self {
			Self::Integer(x) => Ok(x.clone()),
			Self::Array(x) => {
				if x.is_empty() {
					return Err(DeserializationErrorKind::UnexpectedEOF)
				}
				DeserializeItem::<i64>::deserialize(&mut x.remove(0))
			}
			_ => Err(DeserializationErrorKind::InvalidType { expected: "Array or Integer", actual: "todo!" })
		}
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<i64, DeserializationErrorKind> {
		match self {
			Self::Table(x) => {
				let mut val = x.remove(key.borrow()).ok_or(DeserializationErrorKind::MissingField)?;
				DeserializeItem::<i64>::deserialize(&mut val)
			},
			_ => Err(DeserializationErrorKind::InvalidType { expected: "Table", actual: "todo!" })
		}
	}
}


macro_rules! impl_de_int {
    ($int: ty) => {
impl DeserializeItem<$int> for Value {
	fn deserialize(&mut self) -> Result<$int, DeserializationErrorKind> {
		DeserializeItem::<i64>::deserialize(self).and_then(|n| { Ok(n as $int) })
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<$int, DeserializationErrorKind> {
		DeserializeItem::<i64>::deserialize_key(self, key).and_then(|n| { Ok(n as $int) })
	}
}
	};
}

impl_de_int!(u8);
impl_de_int!(u16);


impl DeserializeItem<String> for Value {
	fn deserialize(&mut self) -> Result<String, DeserializationErrorKind> {
		match self {
			Self::Array(x) => {
				if x.is_empty() {
					return Err(DeserializationErrorKind::UnexpectedEOF)
				}
				DeserializeItem::deserialize(&mut x.remove(0))
			}
			_ => self.clone().try_into().map_err(Into::into)
		}
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K) -> Result<String, DeserializationErrorKind> {
		match self {
			Self::Table(x) => {
				let val = x.remove(key.borrow()).ok_or(DeserializationErrorKind::MissingField)?;
				val.try_into().map_err(Into::into)
			},
			_ => Err(DeserializationErrorKind::InvalidType { expected: "Table", actual: "todo!" })
		}
	}
}


impl<T> DeserializeItemVarSize<T> for Value where Value: DeserializeItem<T> {
	fn deserialize(&mut self, _size: usize) -> Result<T, DeserializationErrorKind> {
		DeserializeItem::deserialize(self)
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, _size: usize) -> Result<T, DeserializationErrorKind> {
		DeserializeItem::deserialize_key(self, key)
	}
}

impl<T> DeserializeItemAutoSize<T> for Value where Value: DeserializeItem<T> {
	fn deserialize(&mut self, _size_type: SizeType) -> Result<T, DeserializationErrorKind> {
		DeserializeItem::deserialize(self)
	}

	fn deserialize_key<K: Borrow<str>>(&mut self, key: K, _size_type: SizeType) -> Result<T, DeserializationErrorKind> {
		DeserializeItem::deserialize_key(self, key)
	}
}

impl<T> SerializeItemAutoSize<T> for Value where Value: SerializeItem<T> {
	fn serialize(&mut self, item: T, _size_type: SizeType) {
		SerializeItem::serialize(self, item);
	}

	fn serialize_key<K: Borrow<str>>(&mut self, key: K, item: T, _size_type: SizeType) {
		SerializeItem::serialize_key(self, key, item);
	}
}


impl SerializeSerial for Value {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T, _size_type: SizeType) {
		match self {
			Self::Array(x) => {
				x.push(item.serialize())
			}
			_ => {
				let value = replace(self, Value::Array(Array::new()));
				let arr = match self {
					Self::Array(x) => x,
					_ => unreachable!()
				};
				arr.push(value);
				arr.push(item.serialize());
			}
		}
	}

	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T, _size_type: SizeType) {
		match self {
			Self::Table(x) => { x.insert(key.borrow().into(), item.serialize()); },
			Self::Array(x) => {
				if let Some(Self::Table(x)) = x.last_mut() {
					x.insert(key.borrow().into(), item.serialize());
					return
				}
				let mut table = Table::new();
				table.insert(key.borrow().into(), item.serialize());
				x.push(Value::Table(table));
			}
			_ => {
				let value = replace(self, Value::Array(Array::new()));
				let arr = match self {
					Self::Array(x) => x,
					_ => unreachable!()
				};
				arr.push(value);
				let mut table = Table::new();
				table.insert(key.borrow().into(), item.serialize());
				arr.push(Value::Table(table));
			}
		};
	}
}

impl DeserializeSerial for Value {
	fn deserialize<P, T: Deserialize<P>>(&mut self, _size_type: SizeType) -> Result<T, DeserializationError> {
		match self {
			Self::Array(x) => {
				if x.is_empty() {
					return Err(DeserializationError::from(DeserializationErrorKind::UnexpectedEOF))
				}
				T::deserialize(x.remove(0))
			}
			_ => T::deserialize(self.clone())
		}
	}

	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K, _size_type: SizeType) -> Result<T, DeserializationError> {
		match self {
			Self::Table(x) => {
				if let Some(x) = x.remove(key.borrow()) {
					T::deserialize(x)
				} else {
					Err(DeserializationError::new(key.borrow(), DeserializationErrorKind::MissingField))
				}
			}
			_ => Err(DeserializationError::from(DeserializationErrorKind::InvalidType { expected: "Table", actual: "todo!" }))
		}
	}
}

impl ItemAccess for Value {
	const CAN_GET_KEY: bool = true;

	fn empty() -> Self {
		Value::Table(Table::new())
	}
	fn try_get_key(&self) -> Option<&str> {
		match self {
			Value::Table(x) => x.keys().next().and_then(|x| { Some(x.as_str()) }),
			_ => None
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
