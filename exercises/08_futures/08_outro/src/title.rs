use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TicketTitle(String);

#[derive(Debug, thiserror::Error)]
pub enum TicketTitleError {
    #[error("The title cannot be empty")]
    Empty,
    #[error("The title cannot be longer than 50 bytes")]
    TooLong,
}

impl TryFrom<&str> for TicketTitle {
    type Error = TicketTitleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "" => Err(TicketTitleError::Empty),
            v if v.len() > 50 => Err(TicketTitleError::TooLong),
            _ => Ok(TicketTitle(value.to_string())),
        }
    }
}

impl TryFrom<String> for TicketTitle {
    type Error = TicketTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        TicketTitle::try_from(value.as_str())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_try_from_string() {
        let expected_title = raw_valid_title();
        let title = TicketTitle::try_from(expected_title.to_string()).unwrap();
        assert_eq!(title.0, expected_title);
    }

    #[test]
    fn test_try_from_empty_string() {
        let err = TicketTitle::try_from(raw_invalid_empty_title()).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be empty");
    }

    #[test]
    fn test_try_from_long_string() {
        let title = raw_invalid_51_chars_long_title().to_string();
        let err = TicketTitle::try_from(title).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be longer than 50 bytes");
    }

    #[test]
    fn test_deserialize_empty_title_should_fail() {
        let title = serde_json::to_string(raw_invalid_empty_title()).unwrap();
        let err = serde_json::from_str::<TicketTitle>(&title).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be empty");
    }

    #[test]
    fn test_deserialize_title_too_long_should_fail() {
        let title = serde_json::to_string(raw_invalid_51_chars_long_title()).unwrap();
        let err = serde_json::from_str::<TicketTitle>(&title).unwrap_err();
        assert_eq!(err.to_string(), "The title cannot be longer than 50 bytes");
    }

    pub fn raw_valid_title() -> &'static str {
        "Sed ut perspiciatis unde omnis iste natus error si"
    }

    pub fn valid_title() -> TicketTitle {
        TicketTitle::try_from(raw_valid_title()).unwrap()
    }

    pub fn raw_invalid_51_chars_long_title() -> &'static str {
        "Sed ut perspiciatis unde omnis iste natus error sit"
    }

    pub fn raw_invalid_empty_title() -> &'static str {
        ""
    }
}
