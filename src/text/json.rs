use std::collections::{HashMap, VecDeque};
use std::fmt::Write;
use std::ops::Add;
use std::str::FromStr;

use super::*;

pub mod json_prelude {
	pub use crate::{impl_json, impl_json_deser, impl_json_ser};

	pub use super::{JSONDeserialize, JSONSerialize, text::TextRepr};
}


fn split_layer(data: String) -> Result<Vec<String>, char> {
	let mut out = Vec::new();
	let mut curly_count = 0usize;
	let mut square_count = 0usize;
	let mut buffer = String::new();

	for c in data.trim().chars() {
		if c == '{' {
			curly_count += 1;
			if curly_count == 1 {
				continue
			}
		} else if c == '}' {
			if curly_count == 0 {
				return Err(c)
			}
			curly_count -= 1;
			if curly_count == 0 {
				continue
			}
		} else if c == '[' {
			square_count += 1;
			if square_count == 1 {
				continue
			}
		} else if c == ']' {
			if square_count == 0 {
				return Err(c)
			}
			square_count -= 1;
			if square_count == 0 {
				continue
			}
		} else if c == ',' && !(square_count > 1 || curly_count > 1) {
			out.push(buffer.trim().into());
			buffer.clear();
			continue
		}

		buffer.push(c);
	}

	if !buffer.is_empty() {
		out.push(buffer.trim().into());
	}

	Ok(out)
}


impl TextRepr {
	pub fn is_valid_json<T: ToString>(data: T) -> bool {
		Self::from_json(data.to_string()).is_ok()
	}
	pub fn to_json(self) -> String {
		match self {
			TextRepr::Empty => String::new(),
			TextRepr::String(x) => format!("\"{}\"", x),
			TextRepr::Integer(x) => x.to_string(),
			TextRepr::Float(x) => x.to_string(),
			TextRepr::Boolean(x) => x.to_string(),
			TextRepr::Table(x) => {
				let mut out = String::from("{\n");

				for (key, value) in x {
					writeln!(out, "\t{}: {},", key, value.to_json()).expect("Unexpected error while writing to json string. Please report this to the developer");
				}

				out.add("}")
			}
			TextRepr::Array(x) => format!(
				"{:?}",
				x.into_iter().map(Self::to_json).collect::<Vec<_>>()
			)
		}
	}
	pub fn from_json(data: String) -> Result<Self, DeserializationError> {
		let mut out = Self::new();
		let data = data.trim().to_string();
		let chars: VecDeque<_> = data.char_indices().collect();

		let start_char = match chars.front() {
			Some(c) => c.1,
			None => return Err(DeserializationError::EOF)
		};

		if start_char == '{' {
			unsafe {
				if chars.back().unwrap_unchecked().1 != '}' {
					return Err(DeserializationError::invalid_format("missing closing brace"))
				}
			}

			let segments = split_layer(data).map_err(|c| { DeserializationError::invalid_format(format!("Unbalanced braces: {c}")) })?;

			for segment in segments {
				if segment.is_empty() {
					continue
				}
				let idx = match segment.find(':') {
					None => return Err(DeserializationError::invalid_format("missing value").set_field(segment)),
					Some(x) => x
				};
				let (key, value) = segment.split_at(idx);

				let key = key.trim();

				if key.is_empty() {
					return Err(DeserializationError::invalid_format("missing key"))
				}

				let value = match value.get(1..) {
					None => "",
					Some(x) => x.trim()
				};

				if value.is_empty() {
					return Err(DeserializationError::invalid_format("missing value").set_field(key))
				}

				out.push_entry(key.into(), Self::from_json(value.into())?);
			}
		} else if start_char == '[' {
			let segments = split_layer(data).map_err(|c| { DeserializationError::invalid_format(format!("Unbalanced braces: {c}")) })?;

			for segment in segments {
				let segment = segment.trim().to_string();

				if segment.is_empty() {
					return Err(DeserializationError::invalid_format("missing array value"))
				}

				out.push_value(Self::from_json(segment)?);
			}
		} else {
			return Self::from_str_value(data)
		}

		Ok(out)
	}
}


pub trait JSONSerialize<P = NaturalProfile> {
	fn serialize_json(self) -> String;
}


pub trait JSONDeserialize<P = NaturalProfile>: Sized {
	fn deserialize_json(data: String) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into JSON with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait JSONSerde<P = NaturalProfile>: JSONSerialize<P> + JSONDeserialize<P> {}

impl<P, T: JSONSerialize<P> + JSONDeserialize<P>> JSONSerde<P> for T {}


pub trait MarshalledJSONSerialize<Marshall, P = NaturalProfile> {
	fn serialize_json(self, marshall: &Marshall) -> String;
}


pub trait MarshalledJSONDeserialize<'a, Marshall, P = NaturalProfile>: Sized {
	fn deserialize_json(data: String, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
}


#[macro_export]
macro_rules! impl_json {
    ($name: ty, $profile: ty) => {
		impl_json_ser!($name, $profile);
		impl_json_deser!($name, $profile);
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl_json_ser!($name, $profile, $marshall);
		impl_json_deser!($name, $profile, $marshall);
	};
}

#[macro_export]
macro_rules! impl_json_ser {
    ($name: ty, $profile: ty) => {
		impl JSONSerialize<$profile> for $name {
			fn serialize_json(self) -> String {
				let mut out = TextRepr::new();
				Serialize::<$profile>::serialize(self, &mut out);
				out.to_json()
			}
		}
	};
    // ($name: ty, $profile: ty, $($lifetime: lifetime),*) => {
	// 	impl <$($lifetime),*> JSONSerialize<$profile> for $name::<$($lifetime),*> {
	// 		fn serialize_json(self) -> String {
	// 			Serialize::<$profile>::serialize::<JSONSerialized>(self).to_string()
	// 		}
	// 	}
	// };
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledJSONSerialize<$marshall, $profile> for $name {
			fn serialize_json(self, marshall: &$marshall) -> String {
				let mut out = TextRepr::new();
				MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
				out.to_json()
			}
		}
	};
    // ($name: ty, $profile: ty, $marshall: ty, $($lifetime: lifetime),*) => {
	// 	impl<$($lifetime),*> MarshalledJSONSerialize<$marshall, $profile> for $name<$($lifetime),*> {
	// 		fn serialize_json(self, marshall: &$marshall) -> String {
	// 			MarshalledSerialize::<$profile>::serialize::<JSONSerialized>(self, marshall).to_string()
	// 		}
	// 	}
	// };
}

#[macro_export]
macro_rules! impl_json_deser {
    ($name: ty, $profile: ty) => {
		impl JSONDeserialize<$profile> for $name {
			fn deserialize_json(data: String) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize(&mut TextRepr::from_json(data)?)
			}
		}
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledJSONDeserialize<$marshall, $profile> for $name {
			fn deserialize_json(data: String, marshall: &$marshall) -> Result<Self, DeserializationError> {
				MarshalledDeserialize::<$profile>::deserialize(&mut TextRepr::from_json(data)?, marshall)
			}
		}
	};
}


impl<P, K: Borrow<str> + Eq + std::hash::Hash, V: Serialize<P>> JSONSerialize<P> for HashMap<K, V> {
	fn serialize_json(self) -> String {
		TextRepr::to_json(serialize_owned!(self))
	}
}


impl<E: Debug, P, K: Eq + std::hash::Hash + FromStr<Err=E>, V: Deserialize<P>> JSONDeserialize<P> for HashMap<K, V> {
	fn deserialize_json(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize::<TextRepr>(&mut TextRepr::from_json(data)?)
	}
}
