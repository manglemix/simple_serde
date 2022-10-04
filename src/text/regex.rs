use regex::Regex;
use crate::{DeserializationError, DeserializationErrorKind, Deserialize, Serialize, Serializer};


impl Serialize for Regex {
	fn serialize<T: Serializer>(self, data: &mut T) {
		data.serialize_string(self.as_str());
	}
}


impl Deserialize for Regex {
	fn deserialize<T: Serializer>(data: &mut T) -> Result<Self, DeserializationError> {
		Regex::new(data.deserialize_string()?.as_str())
			.map_err(|e| DeserializationError::new_kind(DeserializationErrorKind::RegexParseError(e)))
	}
}
