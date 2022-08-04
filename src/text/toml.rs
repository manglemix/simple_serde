use std::collections::{HashMap, VecDeque};
use std::fmt::Write;

use super::*;

pub(crate) const AVG_TOML_LINE_LENGTH: usize = 30;


pub mod toml_prelude {
	pub use crate::{impl_toml, impl_toml_deser, impl_toml_ser};

	pub use super::{text::TextRepr, TOMLDeserialize, TOMLSerialize};
}


pub(crate) fn map_entries_recursive(map: HashMap<String, TextRepr>, root: Vec<String>, entries: &mut HashMap<Vec<String>, HashMap<String, TextRepr>>) {
	for (key, value) in map {
		match value {
			TextRepr::Table(x) => {
				let mut new_root = root.clone();
				new_root.push(key);
				map_entries_recursive(x, new_root, entries);
			}
			value => {
				match entries.get_mut(&root) {
					None => {
						entries.insert(root.clone(), {
							let mut map = HashMap::new();
							map.insert(key, value);
							map
						});
					},
					Some(map) => { map.insert(key, value); }
				}
			}
		}
	}
}


pub(crate) fn delimit_comma_split(data: &str) -> Vec<String> {
	let mut in_string = false;
	let mut item = String::new();
	let mut out = Vec::new();

	for c in data.chars() {
		if c == '"' {
			in_string = !in_string;
		} else if !in_string && c == ',' {
			out.push(item.clone());
			item.clear();
		}
		item.push(c);
	}

	out
}


impl TextRepr {
	pub fn is_valid_toml<T: ToString>(data: T) -> bool {
		Self::from_toml(data.to_string()).is_ok()
	}
	pub fn to_toml(self) -> String {
		match self {
			TextRepr::Empty => String::new(),
			TextRepr::String(x) => format!("\"{}\"", x),
			TextRepr::Integer(x) => x.to_string(),
			TextRepr::Float(x) => x.to_string(),
			TextRepr::Boolean(x) => x.to_string(),
			TextRepr::Table(map) => {
				let line_count = map.len();
				let mut entries = HashMap::new();
				map_entries_recursive(map, Vec::new(), &mut entries);
				let mut entries: Vec<_> = entries.into_iter().collect();
				entries.sort_by(|x, y| { x.0.len().cmp(&y.0.len()) });

				let mut out = String::with_capacity(AVG_TOML_LINE_LENGTH * line_count);
				for (mut path, values) in entries {
					if !path.is_empty() {
						let mut field_name = path.remove(0);

						for segment in path {
							field_name += ".";
							field_name += segment.as_str();
						}

						writeln!(out, "[{}]", field_name).expect("Error writing map to toml string. Please report this to the developer.");
					}
					for (name, value) in values {
						writeln!(out, "{} = {}", name, value.to_toml()).expect("Error writing map to toml string. Please report this to the developer.");
					}
					out += "\n";
				}
				out.shrink_to_fit();
				out
			}
			TextRepr::Array(x) => {
				debug_assert!(!{
					fn contains_table(arr: &VecDeque<TextRepr>) -> bool {
						for item in arr {
							match item {
								TextRepr::Table(_) => return true,
								TextRepr::Array(arr) => return contains_table(arr),
								_ => {}
							}
						}
						false
					}

					contains_table(&x)
				});
				format!(
					"{:?}",
					x.into_iter().map(Self::to_toml).collect::<Vec<_>>()
				)
			}
		}
	}

	pub fn from_toml(data: String) -> Result<Self, DeserializationError> {
		let mut out = Self::new();
		let mut data: VecDeque<char> = data.chars().collect();
		let mut outer_path = Vec::new();

		while let Some(start_char) = first_symbol(&mut data) {
			if start_char == '[' {
				outer_path.clear();
				let mut segment = String::new();
				loop {
					let c = data.pop_front().ok_or(DeserializationErrorKind::UnexpectedEOF).set_field("Outer Field Name")?;
					if c == ']' {
						break
					}
					if c == '.' {
						outer_path.push(segment.clone());
						segment.clear();
						continue
					}
					segment.push(c);
				}
				if segment.is_empty() {
					// TODO Make clearer
					return Err(DeserializationError::new_kind(DeserializationErrorKind::InvalidFormat { reason: "Outer field name is either empty or terminates incorrectly".into() }))
				}
				outer_path.push(segment);
				continue
			}
			let mut key = String::from(start_char);
			loop {
				let c = data.pop_front().ok_or(DeserializationErrorKind::UnexpectedEOF).set_field(key.clone())?;
				if c == '=' {
					break
				}
				key.push(c);
			}
			key = key.trim().to_string();

			let mut value = String::new();
			while let Some(c) = data.pop_front() {
				if c == '\n' {
					break
				}
				value.push(c);
			}
			value = value.trim().to_string();

			let mut new_path = outer_path.clone();
			new_path.push(key);
			new_path.reverse();

			if value.starts_with('[') {
				let mut arr = VecDeque::new();
				for item in delimit_comma_split(value.get(1..(value.len() - 1)).unwrap()) {
					arr.push_back(Self::from_str_value(item.trim().to_string())?);
				}
				out.push_entry_path(new_path, Self::Array(arr))
			} else {
				out.push_entry_path(new_path, Self::from_str_value(value)?);
			}
		}

		Ok(out)
	}
}


pub trait TOMLSerialize<P = NaturalProfile> {
	fn serialize_toml(self) -> String;
}


pub trait TOMLDeserialize<P = NaturalProfile>: Sized {
	fn deserialize_toml(data: String) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into TOML with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait TOMLSerde<P = NaturalProfile>: TOMLSerialize<P> + TOMLDeserialize<P> {}

impl<P, T: TOMLSerialize<P> + TOMLDeserialize<P>> TOMLSerde<P> for T {}


pub trait MarshalledTOMLSerialize<Marshall, P = NaturalProfile> {
	fn serialize_toml(self, marshall: &Marshall) -> String;
}


pub trait MarshalledTOMLDeserialize<'a, Marshall, P = NaturalProfile>: Sized {
	fn deserialize_toml(data: String, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into TOML with the same profile,
/// and the same type of marshall. Is automatically implemented on all appropriate types
pub trait MarshalledTOMLSerde<'a, Marshall, P = NaturalProfile>: MarshalledTOMLSerialize<Marshall, P> + MarshalledTOMLDeserialize<'a, Marshall, P> {}

impl<'a, P, Marshall, T: MarshalledTOMLSerialize<Marshall, P> + MarshalledTOMLDeserialize<'a, Marshall, P>> MarshalledTOMLSerde<'a, Marshall, P> for T {}

#[macro_export]
macro_rules! impl_toml {
    ($name: ty, $profile: ty) => {
		impl_toml_ser!($name, $profile);
		impl_toml_deser!($name, $profile);
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl_toml_ser!($name, $profile, $marshall);
		impl_toml_deser!($name, $profile, $marshall);
	};
}

#[macro_export]
macro_rules! impl_toml_ser {
    ($name: ty, $profile: ty) => {
		impl TOMLSerialize<$profile> for $name {
			fn serialize_toml(self) -> String {
				let mut out = TextRepr::new();
				Serialize::<$profile>::serialize(self, &mut out);
				out.to_toml()
			}
		}
	};
    // ($name: ty, $profile: ty, $($lifetime: lifetime),*) => {
	// 	impl <$($lifetime),*> TOMLSerialize<$profile> for $name::<$($lifetime),*> {
	// 		fn serialize_toml(self) -> String {
	// 			Serialize::<$profile>::serialize::<TOMLSerialized>(self).to_string()
	// 		}
	// 	}
	// };
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledTOMLSerialize<$marshall, $profile> for $name {
			fn serialize_toml(self, marshall: &$marshall) -> String {
				let mut out = TextRepr::new();
				MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
				out.to_toml()
			}
		}
	};
    // ($name: ty, $profile: ty, $marshall: ty, $($lifetime: lifetime),*) => {
	// 	impl<$($lifetime),*> MarshalledTOMLSerialize<$marshall, $profile> for $name<$($lifetime),*> {
	// 		fn serialize_toml(self, marshall: &$marshall) -> String {
	// 			MarshalledSerialize::<$profile>::serialize::<TOMLSerialized>(self, marshall).to_string()
	// 		}
	// 	}
	// };
}

#[macro_export]
macro_rules! impl_toml_deser {
    ($name: ty, $profile: ty) => {
		impl TOMLDeserialize<$profile> for $name {
			fn deserialize_toml(data: String) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize(&mut TextRepr::from_toml(data)?)
			}
		}
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledTOMLDeserialize<$marshall, $profile> for $name {
			fn deserialize_toml(data: String, marshall: &$marshall) -> Result<Self, DeserializationError> {
				MarshalledDeserialize::<$profile>::deserialize(&mut TextRepr::from_toml(data)?, marshall)
			}
		}
	};
}


impl<P, K: Borrow<str> + Eq + std::hash::Hash, V: Serialize<P>> TOMLSerialize<P> for HashMap<K, V> {
	fn serialize_toml(self) -> String {
		TextRepr::to_toml(serialize_owned!(self))
	}
}


impl<P, K: Eq + std::hash::Hash + From<String>, V: Deserialize<P>> TOMLDeserialize<P> for HashMap<K, V> {
	fn deserialize_toml(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize::<TextRepr>(&mut TextRepr::from_toml(data)?)
	}
}
