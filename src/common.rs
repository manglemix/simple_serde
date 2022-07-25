use std::collections::HashMap;
use std::hash::Hash;
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


impl<P, K, V> Deserialize<P> for HashMap<K, V>
	where
		K: Eq + Hash + From<String>,
		V: Deserialize<P>
{
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		let mut out = Self::new();
		while let Some(key) = data.try_get_key() {
			out.insert(K::from(key.clone()), data.deserialize_key(key)?);
		}
		Ok(out)
	}
}


impl<V: Serialize> Serialize for Vec<V> {
	fn serialize<T: Serializer>(self, data: &mut T) {
		for item in self {
			data.serialize(item);
		}
	}
}


impl<V: Deserialize> Deserialize for Vec<V> {
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
