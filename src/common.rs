use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::str::FromStr;

use super::*;

impl<P, K, V> Serialize<P> for HashMap<K, V>
	where
		K: Borrow<str> + Eq + Hash,
		V: Serialize<P>
{
	fn serialize<T: Serializer>(self, data: &mut T) {
		for (key, val) in self {
			data.serialize_key(key.borrow(), val);
		}
	}
}


impl<P, K, V, E> Deserialize<P> for HashMap<K, V>
	where
		E: Debug,
		K: Eq + Hash + FromStr<Err=E>,
		V: Deserialize<P>
{
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		let mut out = Self::new();
		while let Some(key) = data.try_get_key::<String>() {
			let deser = data.deserialize_key(key.as_str()).set_field(key.clone())?;
			out.insert(K::from_str(key.as_str()).map_err(|e| DeserializationError::new(key, DeserializationErrorKind::from_str_err(e)))?, deser);
		}

		Ok(out)
	}
}


impl<P, V: Serialize<P>> Serialize<P> for Vec<V> {
	fn serialize<T: Serializer>(self, data: &mut T) {
		for item in self {
			data.serialize(item);
		}
	}
}


impl<P, V: Deserialize<P>> Deserialize<P> for Vec<V> {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		let data_ref = data.borrow_mut();
		let mut out = Self::new();
		loop {
			match data_ref.deserialize() {
				Ok(x) => out.push(x),
				Err(e) => match &e.kind {
					DeserializationErrorKind::UnexpectedEOF => break,
					_ => return Err(e)
				}
			}
		}
		Ok(out)
	}
}


impl<P, V: Serialize<P> + Eq + Hash> Serialize<P> for HashSet<V> {
	fn serialize<T: Serializer>(self, data: &mut T) {
		for item in self {
			data.serialize(item);
		}
	}
}


impl<P, V: Deserialize<P> + Eq + Hash> Deserialize<P> for HashSet<V> {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		let data_ref = data.borrow_mut();
		let mut out = Self::new();
		loop {
			match data_ref.deserialize() {
				Ok(x) => out.insert(x),
				Err(e) => match &e.kind {
					DeserializationErrorKind::UnexpectedEOF => break,
					_ => return Err(e)
				}
			};
		}
		Ok(out)
	}
}
