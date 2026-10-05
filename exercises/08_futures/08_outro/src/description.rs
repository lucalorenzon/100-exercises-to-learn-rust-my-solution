#[derive(Debug, PartialEq, Clone)] // unwrapp_err => Debug, assert_eq => Partial_Eq, Ticket.clone() => Clone
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
            v if v.is_empty() => Err(TicketDescriptionError::Empty),
            v if v.len() > 50 => Err(TicketDescriptionError::TooLong),
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
        let description = raw_invalid_verly_long_description().to_string();
        let err = TicketDescription::try_from(description).unwrap_err();
        assert_eq!(
            err.to_string(),
            "The description cannot be longer than 500 bytes"
        );
    }

    pub fn raw_invalid_verly_long_description() -> String {
        "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga. Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore, cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime placeat facere possimus, omnis voluptas assumenda est, omnis dolor repellendus. Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet ut et voluptates repudiandae sint et molestiae non recusandae. Itaque earum rerum hic tenetur a sapiente delectus, ut aut reiciendis voluptatibus maiores alias consequatur aut perferendis doloribus asperiores repellat.".into()
    }

    pub fn raw_valid_description() -> String {
        "A description".into()
    }

    pub fn valid_description() -> TicketDescription {
        TicketDescription::try_from(raw_valid_description()).unwrap()
    }

    pub fn raw_invalid_empty_description() -> String {
        "".into()
    }
}
