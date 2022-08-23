use std::collections::{HashMap, VecDeque};
use std::hint;
use std::mem::replace;

pub use json::json_prelude;
pub use toml::toml_prelude;
pub use mlist::mlist_prelude;

use super::*;

pub mod toml;
pub mod json;
pub mod mlist;


macro_rules! serialize_owned {
    ($item: expr) => {{
		let mut owner = TextRepr::new();
		$item.serialize(&mut owner);
		owner
	}};
}

use serialize_owned;


#[derive(Debug)]
pub enum TextRepr {
	Empty,
	String(String),
	Integer(i64),
	Float(f64),
	Boolean(bool),
	Table(HashMap<String, Self>),
	Array(VecDeque<TextRepr>),
}


fn first_symbol(data: &mut VecDeque<char>) -> Option<char> {
	while let Some(c) = data.pop_front() {
		match c {
			'\n' | ' ' | '\t' | '\r' => {},
			c => return Some(c)
		}
	}
	None
}


impl TextRepr {
	pub fn new() -> Self {
		Self::Empty
	}

	pub fn is_empty(&self) -> bool {
		match self {
			Self::Empty => true,
			Self::Table(x) => x.is_empty(),
			Self::Array(x) => x.is_empty(),
			_ => false
		}
	}

	pub fn pull_value(&mut self) -> Result<Self, DeserializationErrorKind> {
		match self {
			TextRepr::Array(x) => x.pop_front().ok_or(DeserializationErrorKind::UnexpectedEOF),
			// TextRepr::Table(_) => Err(DeserializationErrorKind::InvalidType { expected: "non-table", actual: "table" }),
			_ => Ok(replace(self, Self::Empty))
		}
	}

	pub fn pull_entry<T: Borrow<String>>(&mut self, key: T) -> Result<Self, DeserializationErrorKind> {
		match self {
			TextRepr::Table(x) => x.remove(key.borrow()).ok_or(DeserializationErrorKind::MissingField),
			_ => Err(DeserializationErrorKind::InvalidType { expected: "table", actual: "non-table" })
		}
	}

	fn push_value(&mut self, other: Self) {
		match self {
			TextRepr::Empty => *self = other,
			TextRepr::Array(x) => x.push_back(other),
			TextRepr::Table(_) => panic!("Tried to push a TextRepr onto a table TextRepr!"),
			_ => {
				let value = replace(self, Self::Array(VecDeque::new()));
				match self {
					Self::Array(arr) => {
						arr.push_front(value);
						arr.push_front(other);
					}
					_ => unreachable!()
				}
			}
		};
	}

	fn push_entry(&mut self, key: String, other: Self) {
		match self {
			TextRepr::Empty => {
				let mut table = HashMap::new();
				table.insert(key, other);
				*self = Self::Table(table);
			}
			TextRepr::Table(x) => { x.insert(key, other); }
			_ => panic!("Tried to insert a TextRepr onto a non-empty and non-table TextRepr!")
		}
	}

	fn push_entry_path(&mut self, mut path: Vec<String>, other: Self) {
		assert!(!path.is_empty());
		if path.len() == 1 {
			self.push_entry(path.pop().unwrap(), other);
			return
		}
		match self {
			TextRepr::Empty => {
				*self = Self::Table(HashMap::new());
				self.push_entry_path(path, other);
			}
			TextRepr::Table(x) => {
				let field_name = path.pop().unwrap();
				if !x.contains_key(&field_name) {
					x.insert(field_name.clone(), TextRepr::Table(HashMap::new()));
				}
				x.get_mut(&field_name).unwrap().push_entry_path(path, other);
			}
			_ => panic!("Tried to insert a TextRepr onto a non-empty and non-table TextRepr!")
		}
	}

	fn from_str_value(mut data: String) -> Result<Self, DeserializationError> {
		if data.is_empty() {
			return Err(DeserializationError::new_kind(DeserializationErrorKind::UnexpectedEOF))
		}

		if data.starts_with('"') {
			if !data.ends_with('"') {
				return Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidFormat { reason: "String is missing terminating apostrophe".into() }))
			}

			return Ok(TextRepr::String(data.drain(1..(data.len() - 1)).collect()))
		}
		if data.ends_with('"') {
			return Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidFormat { reason: "String is missing starting apostrophe".into() }))
		}

		macro_rules! try_or_skip {
				($variant: ident) => {
					match data.parse() {
						Ok(x) => return Ok(TextRepr::$variant(x)),
						Err(_) => {}
					}
				};
			}

		try_or_skip!(Boolean);
		try_or_skip!(Integer);
		try_or_skip!(Float);
		Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "todo!", actual: "todo!" }))
	}
}


impl PrimitiveSerializer for TextRepr {
	fn serialize_bool(&mut self, boolean: bool) {
		self.push_value(TextRepr::Boolean(boolean));
	}

	fn deserialize_bool(&mut self) -> Result<bool, DeserializationError> {
		match self.pull_value().no_field()? {
			TextRepr::Boolean(x) => Ok(x),
			_ => Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "number", actual: "todo!" }))
		}
	}

	fn serialize_num<T: NumberType>(&mut self, num: T) {
		self.push_value(num.to_text());
	}

	fn deserialize_num<T: NumberType>(&mut self) -> Result<T, DeserializationError> {
		match self.pull_value().no_field()? {
			TextRepr::Integer(x) => T::from_i64(x).ok_or_else(|| DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "unsigned int", actual: "signed int" })),
			TextRepr::Float(x) => T::from_f64(x).ok_or_else(|| DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "integer", actual: "float" })),
			_ => Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "number", actual: "todo!" }))
		}
	}

	fn serialize_string<T: Into<String>>(&mut self, string: T) {
		self.push_value(Self::String(string.into()));
	}

	fn deserialize_string(&mut self) -> Result<String, DeserializationError> {
		match self.pull_value().no_field()? {
			TextRepr::String(x) => Ok(x),
			_ => Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "string", actual: "todo!" }))
		}
	}

	fn serialize_bytes<T: Into<VecDeque<u8>>>(&mut self, bytes: T) {
		let bytes = bytes.into();
		self.push_value(Self::Array(bytes.into_iter().map(|x| Self::Integer(x as i64)).collect()));
	}

	fn deserialize_bytes<T: FromIterator<u8>>(&mut self) -> Result<T, DeserializationError> {
		unsafe {
			match self {
				Self::Array(_) => {
					let values = match replace(self, Self::Empty) {
						Self::Array(arr) => arr,
						_ => hint::unreachable_unchecked()
					};
					let mut out = Vec::with_capacity(values.len());

					for value in values {
						match value {
							Self::Integer(x) => out.push(x as u8),
							_ => return Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "byte", actual: "todo!" }))
						}
					}

					Ok(out.into_iter().collect())
				},
				_ => Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidType { expected: "array", actual: "todo!" }))
			}
		}
	}
}

impl Serializer for TextRepr {
	fn serialize<P, T: Serialize<P>>(&mut self, item: T) {
		self.push_value(serialize_owned!(item));
	}

	fn serialize_key<P, T: Serialize<P>, K: Borrow<str>>(&mut self, key: K, item: T) {
		self.push_entry(key.borrow().into(), serialize_owned!(item));
	}

	fn deserialize<P, T: Deserialize<P>>(&mut self) -> Result<T, DeserializationError> {
		let mut value = self.pull_value().no_field()?;
		let result = T::deserialize(&mut value);
		if !value.is_empty() {
			self.push_value(value);
		}
		result
	}

	fn deserialize_key<P, T: Deserialize<P>, K: Borrow<str>>(&mut self, key: K) -> Result<T, DeserializationError> {
		let key: String = key.borrow().into();
		let mut value = self.pull_entry(key.clone()).set_field(key.clone())?;
		let result = T::deserialize(&mut value).map_err(|e| { e.nest().set_field(key.clone()) });
		if !value.is_empty() {
			self.push_entry(key, value);
		}
		result
	}

	fn try_get_key<K: FromStr>(&mut self) -> Option<K> {
		match self {
			Self::Table(x) => x.keys().next().map(|x| K::from_str(x.as_str()).ok()).flatten(),
			_ => None
		}
	}
}
