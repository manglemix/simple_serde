use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use std::fmt::Write;

use super::*;

pub mod json_prelude {
	pub use super::{JSONSerialize, JSONDeserialize, text::TextRepr};
	pub use crate::{impl_json, impl_json_ser, impl_json_deser};
}


impl TextRepr {
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
		let mut data: VecDeque<char> = data.chars().collect();

		let start_char = match first_symbol(&mut data) {
			Some(c) => c,
			None => return Ok(out)
		};

		if start_char == '{' {
			'outer: loop {
				let mut key = String::from(match first_symbol(&mut data) {
					Some(c) => c,
					None => continue
				});
				loop {
					let c = match data.pop_front() {
						Some(c) => c,
						None => break 'outer
					};
					if c == ':' {
						break
					}
					key.push(c);
				}
				key = key.trim().to_string();

				let mut value = String::new();
				let mut ended = false;
				loop {
					let c = match data.pop_front() {
						Some(c) => c,
						None => return Err(DeserializationError::new_kind(DeserializationErrorKind::UnexpectedEOF))
					};
					if c == ',' {
						break
					}
					if c == '}' {
						ended = true;
						break
					}
					value.push(c);
				}
				value = value.trim().to_string();
				out.push_entry(key, Self::from_json(value)?);
				if ended {
					break
				}
			}

		} else if start_char == '[' {
			let mut value = String::new();
			while let Some(c) = data.pop_front() {
				if c == ',' {
					let value = value.trim().to_string();
					out.push_value(Self::from_json(value)?);
					continue
				}
				if c == ']' {
					break
				}
				value.push(c);
			}

		} else {
			data.push_front(start_char);
			return Self::from_str_value(data.into_iter().collect())
		}

		Ok(out)
	}
}


pub trait JSONSerialize<P=NaturalProfile> {
	fn serialize_json(self) -> String;
}


pub trait JSONDeserialize<P=NaturalProfile>: Sized {
	fn deserialize_json(data: String) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into JSON with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait JSONSerde<P=NaturalProfile>: JSONSerialize<P> + JSONDeserialize<P> {}
impl<P, T: JSONSerialize<P> + JSONDeserialize<P>> JSONSerde<P> for T {}


pub trait MarshalledJSONSerialize<Marshall, P=NaturalProfile> {
	fn serialize_json(self, marshall: &Marshall) -> String;
}


pub trait MarshalledJSONDeserialize<'a, Marshall, P=NaturalProfile>: Sized {
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


impl<P, K: Eq + std::hash::Hash + From<String>, V: Deserialize<P>> JSONDeserialize<P> for HashMap<K, V> {
	fn deserialize_json(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize::<TextRepr>(&mut TextRepr::from_json(data)?)
	}
}
