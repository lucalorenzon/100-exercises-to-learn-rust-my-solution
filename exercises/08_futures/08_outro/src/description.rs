use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TicketDescription(String);

#[derive(Debug, thiserror::Error)]
pub enum TicketDescriptionError {
    #[error("The description cannot be empty")]
    Empty,
    #[error("The description cannot be longer than 500 bytes")]
    TooLong,
}

impl TryFrom<&str> for TicketDescription {
    type Error = TicketDescriptionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "" => Err(TicketDescriptionError::Empty),
            v if v.len() > 500 => Err(TicketDescriptionError::TooLong),
            _ => Ok(TicketDescription(value.to_string())),
        }
    }
}

impl TryFrom<String> for TicketDescription {
    type Error = TicketDescriptionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        TicketDescription::try_from(value.as_str())
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_try_from_string() {
        let expected_description = raw_valid_description();
        let description = TicketDescription::try_from(expected_description.to_string()).unwrap();
        assert_eq!(description.0, expected_description);
    }

    #[test]
    fn test_try_from_empty_string() {
        let err = TicketDescription::try_from(raw_invalid_empty_description()).unwrap_err();
        assert_eq!(err.to_string(), "The description cannot be empty");
    }

    #[test]
    fn test_try_from_long_string() {
        let description = raw_invalid_very_long_description().to_string();
        let err = TicketDescription::try_from(description).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The description cannot be longer than 500 bytes"
        );
    }

    #[test]
    fn test_deserialize_empty_description_should_fail() {
        let description = &raw_invalid_empty_description();
        let description_json = serde_json::to_string(description).unwrap();
        let err = serde_json::from_str::<TicketDescription>(&description_json).unwrap_err();
        assert_eq!(err.to_string(), "The description cannot be empty");
    }

    #[test]
    fn test_deserialize_description_too_long_should_fail() {
        let description = &raw_invalid_very_long_description();
        let description_json = serde_json::to_string(description).unwrap();
        let err = serde_json::from_str::<TicketDescription>(&description_json).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The description cannot be longer than 500 bytes"
        );
    }

    pub fn raw_invalid_very_long_description() -> String {
        "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempo".into()
    }

    pub fn raw_valid_description() -> String {
        "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi temp".into()
    }

    pub fn valid_description() -> TicketDescription {
        TicketDescription::try_from(raw_valid_description()).unwrap()
    }

    pub fn raw_invalid_empty_description() -> String {
        "".into()
    }
}
