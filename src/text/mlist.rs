use std::collections::{HashMap, VecDeque};
use std::fmt::Write;
use std::str::FromStr;
use crate::toml::{AVG_TOML_LINE_LENGTH, map_entries_recursive};

use super::*;


pub mod mlist_prelude {
	pub use crate::{impl_mlist, impl_mlist_deser, impl_mlist_ser};

	pub use super::{text::TextRepr, MListDeserialize, MListSerialize};
}


impl TextRepr {
	pub fn is_valid_mlist<T: ToString>(data: T) -> bool {
		Self::from_mlist(data.to_string()).is_ok()
	}
	pub fn to_mlist(self) -> String {
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
					let mut field_name;
					if path.is_empty() {
						field_name = String::new();
					} else {
						field_name = path.remove(0);

						for segment in path {
							field_name += ".";
							field_name += segment.as_str();
						}
					}
					for (name, value) in values {
						writeln!(out, "[{}]\n{}", field_name.clone() + name.as_str(), value.to_mlist()).expect("Error writing map to mlist string. Please report this to the developer.");
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
				let mut out = String::new();

				for v in x {
					writeln!(out, "{}", v.to_mlist()).expect("Error writing map to mlist string. Please report this to the developer.");
				}

				out
			}
		}
	}

	pub fn from_mlist(data: String) -> Result<Self, DeserializationError> {
		let mut out = Self::new();
		let mut data: VecDeque<char> = data.chars().collect();
		let mut outer_path = Vec::new();
		let mut values = Vec::new();

		while let Some(start_char) = first_symbol(&mut data) {
			if start_char == '[' {
				if !values.is_empty() {
					let mut new_path = outer_path.clone();
					new_path.reverse();
					out.push_entry_path(new_path, Self::Array(values.into()));
					values = Vec::new();
				}

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

			let mut value = String::from(start_char);
			while let Some(c) = data.pop_front() {
				if c == '\n' {
					break
				}
				value.push(c);
			}
			value = value.trim().to_string();
			values.push(Self::from_str_value(value)?);
		}

		if !values.is_empty() {
			let mut new_path = outer_path.clone();
			new_path.reverse();
			out.push_entry_path(new_path, Self::Array(values.into()));
		}

		Ok(out)
	}
}


pub trait MListSerialize<P = NaturalProfile> {
	fn serialize_mlist(self) -> String;
}


pub trait MListDeserialize<P = NaturalProfile>: Sized {
	fn deserialize_mlist(data: String) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into MList with the same profile,
/// without a marshall. Is automatically implemented on all appropriate types
pub trait MListSerde<P = NaturalProfile>: MListSerialize<P> + MListDeserialize<P> {}

impl<P, T: MListSerialize<P> + MListDeserialize<P>> MListSerde<P> for T {}


pub trait MarshalledMListSerialize<Marshall, P = NaturalProfile> {
	fn serialize_mlist(self, marshall: &Marshall) -> String;
}


pub trait MarshalledMListDeserialize<'a, Marshall, P = NaturalProfile>: Sized {
	fn deserialize_mlist(data: String, marshall: &'a Marshall) -> Result<Self, DeserializationError>;
}


/// A marker trait for types that can be serialized and deserialized into MList with the same profile,
/// and the same type of marshall. Is automatically implemented on all appropriate types
pub trait MarshalledMListSerde<'a, Marshall, P = NaturalProfile>: MarshalledMListSerialize<Marshall, P> + MarshalledMListDeserialize<'a, Marshall, P> {}

impl<'a, P, Marshall, T: MarshalledMListSerialize<Marshall, P> + MarshalledMListDeserialize<'a, Marshall, P>> MarshalledMListSerde<'a, Marshall, P> for T {}

#[macro_export]
macro_rules! impl_mlist {
    ($name: ty, $profile: ty) => {
		impl_mlist_ser!($name, $profile);
		impl_mlist_deser!($name, $profile);
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl_mlist_ser!($name, $profile, $marshall);
		impl_mlist_deser!($name, $profile, $marshall);
	};
}

#[macro_export]
macro_rules! impl_mlist_ser {
    ($name: ty, $profile: ty) => {
		impl MListSerialize<$profile> for $name {
			fn serialize_mlist(self) -> String {
				let mut out = TextRepr::new();
				Serialize::<$profile>::serialize(self, &mut out);
				out.to_mlist()
			}
		}
	};
    // ($name: ty, $profile: ty, $($lifetime: lifetime),*) => {
	// 	impl <$($lifetime),*> MListSerialize<$profile> for $name::<$($lifetime),*> {
	// 		fn serialize_mlist(self) -> String {
	// 			Serialize::<$profile>::serialize::<MListSerialized>(self).to_string()
	// 		}
	// 	}
	// };
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledMListSerialize<$marshall, $profile> for $name {
			fn serialize_mlist(self, marshall: &$marshall) -> String {
				let mut out = TextRepr::new();
				MarshalledSerialize::<$profile>::serialize(self, &mut out, marshall);
				out.to_mlist()
			}
		}
	};
    // ($name: ty, $profile: ty, $marshall: ty, $($lifetime: lifetime),*) => {
	// 	impl<$($lifetime),*> MarshalledMListSerialize<$marshall, $profile> for $name<$($lifetime),*> {
	// 		fn serialize_mlist(self, marshall: &$marshall) -> String {
	// 			MarshalledSerialize::<$profile>::serialize::<MListSerialized>(self, marshall).to_string()
	// 		}
	// 	}
	// };
}

#[macro_export]
macro_rules! impl_mlist_deser {
    ($name: ty, $profile: ty) => {
		impl MListDeserialize<$profile> for $name {
			fn deserialize_mlist(data: String) -> Result<Self, DeserializationError> {
				Deserialize::<$profile>::deserialize(&mut TextRepr::from_mlist(data)?)
			}
		}
	};
    ($name: ty, $profile: ty, $marshall: ty) => {
		impl MarshalledMListDeserialize<$marshall, $profile> for $name {
			fn deserialize_mlist(data: String, marshall: &$marshall) -> Result<Self, DeserializationError> {
				MarshalledDeserialize::<$profile>::deserialize(&mut TextRepr::from_mlist(data)?, marshall)
			}
		}
	};
}


impl<P, K: Borrow<str> + Eq + std::hash::Hash, V: Serialize<P>> MListSerialize<P> for HashMap<K, V> {
	fn serialize_mlist(self) -> String {
		TextRepr::to_mlist(serialize_owned!(self))
	}
}


impl<E: Debug, P, K: Eq + std::hash::Hash + FromStr<Err=E>, V: Deserialize<P>> MListDeserialize<P> for HashMap<K, V> {
	fn deserialize_mlist(data: String) -> Result<Self, DeserializationError> {
		Self::deserialize::<TextRepr>(&mut TextRepr::from_mlist(data)?)
	}
}
