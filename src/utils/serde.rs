use serde::{Deserializer, de};
use std::fmt;

/// Custom deserializer for Option<Vec<String>> that handles single-string sources
/// (like environment variables) by splitting on newlines.
pub fn deserialize_opt_vec_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    // A Visitor to handle the various data types Serde might present.
    struct VecStringVisitor;

    impl<'de> de::Visitor<'de> for VecStringVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a sequence of strings")
        }

        // Case 1: Handle the single string received from the environment variable.
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let commands: Vec<String> = value
                .split('\n') // Split by newlines (our primary delimiter)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()) // Filter out blank lines
                .collect();

            if commands.is_empty() {
                Ok(None)
            } else {
                Ok(Some(commands))
            }
        }

        // Case 2: Handle a sequence (e.g., a JSON array), for compatibility.
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(element) = seq.next_element()? {
                vec.push(element);
            }
            if vec.is_empty() {
                Ok(None)
            } else {
                Ok(Some(vec))
            }
        }

        // Case 3: Handle null/None explicitly.
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        // <-- Note the added <E> here
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    // Direct the deserializer to use our custom visitor.
    deserializer.deserialize_any(VecStringVisitor)
}
