// TODO: Implement `TryFrom<String>` and `TryFrom<&str>` for `Status`.
//  The parsing should be case-insensitive.

#[derive(Debug, PartialEq, Clone)]
enum Status {
    ToDo,
    InProgress,
    Done,
}

#[derive(Debug, thiserror::Error)]
#[error("Status can be only one of ToDo InProgress Done and it is case insensitive")]
struct StatusInvalidError;

impl TryFrom<String> for Status {
    type Error = StatusInvalidError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Status::try_from(value.as_str())
    }
}

impl TryFrom<&str> for Status {
    type Error = StatusInvalidError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            v if v.eq_ignore_ascii_case("todo") => Ok(Status::ToDo),
            v if v.eq_ignore_ascii_case("inprogress") => Ok(Status::InProgress),
            v if v.eq_ignore_ascii_case("done") => Ok(Status::Done),
            _ => Err(StatusInvalidError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn test_try_from_string() {
        let status = Status::try_from("ToDO".to_string()).unwrap();
        assert_eq!(status, Status::ToDo);

        let status = Status::try_from("inproGress".to_string()).unwrap();
        assert_eq!(status, Status::InProgress);

        let status = Status::try_from("Done".to_string()).unwrap();
        assert_eq!(status, Status::Done);

        let empty = String::new();
        assert!(Status::try_from(empty).is_err());

        let invalid = "InValid".to_string();
        assert!(Status::try_from(invalid).is_err());
    }

    #[test]
    fn test_try_from_str() {
        let status = Status::try_from("todo").unwrap();
        assert_eq!(status, Status::ToDo);

        let status = Status::try_from("inprogress").unwrap();
        assert_eq!(status, Status::InProgress);

        let status = Status::try_from("done").unwrap();
        assert_eq!(status, Status::Done);

        assert!(Status::try_from("").is_err());
        assert!(Status::try_from("invalid").is_err());
    }
}
