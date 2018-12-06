extern crate lambda_runtime as lambda;
extern crate serde_derive;
extern crate rusoto_core;
extern crate rusoto_dynamodb;

use std::error::Error;
use serde_derive::{Serialize, Deserialize};
use lambda::{lambda, Context, error::HandlerError};

use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient, GetItemInput, AttributeValue};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn Error>> {
    lambda!(validation_handler);
    Ok(())
}

fn validation_handler(event: ValidationEvent, _ctx: Context) -> Result<ValidationResult, HandlerError> {
    Ok(validate_serial(event.serial_number.as_str()))
}

enum ValidationError {
    InvalidFormat,
    AlreadyExists
}

impl ValidationError {
    fn value(&self) -> String {
        match *self {
            ValidationError::InvalidFormat => String::from("invalid_format"),
            ValidationError::AlreadyExists => String::from("already_exists"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ValidationResult {
    #[serde(rename = "isValid")]
    is_valid: bool,
    errors: Vec<String>
}

#[derive(Serialize, Deserialize)]
struct ValidationEvent {
    #[serde(rename = "serialNumber")]
    serial_number: String
}

fn validate_serial(serial_number: &str) -> ValidationResult {
    let mut result = ValidationResult { is_valid: true, errors: Vec::new() };

    if !validate_serial_length(serial_number) {
        result.is_valid = false;
        result.errors.push(ValidationError::InvalidFormat.value());
    }

    if !validate_serial_alphanumeric(serial_number) {
        result.is_valid = false;
        result.errors.push(ValidationError::InvalidFormat.value());
    }

    if !validate_serial_unique(serial_number) {
        result.is_valid = false;
        result.errors.push(ValidationError::AlreadyExists.value());
    }

    return result;
}

fn validate_serial_length(serial_number: &str) -> bool {
    serial_number.chars().count() >= 6
}

fn validate_serial_alphanumeric(serial_number: &str) -> bool {
    serial_number.chars().all(char::is_alphanumeric)
}

fn validate_serial_unique(serial_number: &str) -> bool {
    let mut query_key: HashMap<String, AttributeValue> = HashMap::new();
    query_key.insert(String::from("serial_number"), AttributeValue {
        s: Some(serial_number.to_string()),
        ..Default::default()
    });

    let query_serials = GetItemInput {
        key: query_key,
        table_name: String::from("assets"),
        ..Default::default()
    };

    let client = DynamoDbClient::new(Region::EuCentral1);

    match client.get_item(query_serials).sync() {
        Ok(result) => {
            match result.item {
                Some(_) => false, // invalid, serial_number was found
                None => true // valid, serial_number was not found
            }
        },
        Err(error) => {
            panic!("Error: {:?}", error);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_result_for_invalid_length() {
        let test_serial = "i234";
        let validation_result = validate_serial(test_serial);
        assert_eq!(false, validation_result.is_valid);
        assert_eq!(true, validation_result.errors.contains(&String::from("invalid_format")))
    }

    #[test]
    fn validation_result_for_invalid_characters() {
        let test_serial = "i234@";
        let validation_result = validate_serial(test_serial);
        assert_eq!(false, validation_result.is_valid);
        assert_eq!(true, validation_result.errors.contains(&String::from("invalid_format")))
    }

    #[test]
    fn validation_result_for_already_existing_serial() {
        let test_serial = "serial1";
        let validation_result = validate_serial(test_serial);
        assert_eq!(false, validation_result.is_valid);
        assert_eq!(true, validation_result.errors.contains(&String::from("already_exists")))
    }

    #[test]
    fn validation_result_for_valid_serial() {
        let test_serial = "a12345bbc";
        let validation_result = validate_serial(test_serial);
        assert_eq!(true, validation_result.is_valid);
        assert_eq!(true, validation_result.errors.is_empty())
    }

    #[test]
    fn validates_length_of_four_characters_as_invalid() {
        let test_serial = "i234";
        let validation_result = validate_serial_length(test_serial);
        assert_eq!(false, validation_result);
    }

    #[test]
    fn validates_length_of_six_characters_as_valid() {
        let test_serial = "i23456";
        let validation_result = validate_serial_length(test_serial);
        assert_eq!(true, validation_result);
    }

    #[test]
    fn validates_length_of_ten_characters_as_valid() {
        let test_serial = "i234567891";
        let validation_result = validate_serial_length(test_serial);
        assert_eq!(true, validation_result);
    }

    #[test]
    fn validates_string_with_numbers_as_valid() {
        let test_serial = "234567891";
        let validation_result = validate_serial_alphanumeric(test_serial);
        assert_eq!(true, validation_result);
    }

    #[test]
    fn validates_string_with_az_characters_as_valid() {
        let test_serial = "abcd1234";
        let validation_result = validate_serial_alphanumeric(test_serial);
        assert_eq!(true, validation_result);
    }

    #[test]
    fn validates_string_with_unicode_characters_as_valid() {
        let test_serial = "абвгдежзийюя1234";
        let validation_result = validate_serial_alphanumeric(test_serial);
        assert_eq!(true, validation_result);
    }

    #[test]
    fn validates_string_with_special_characters_as_invalid() {
        let test_serial = "abcd!1234";
        let validation_result = validate_serial_alphanumeric(test_serial);
        assert_eq!(false, validation_result);
    }

    #[test]
    fn validates_existing_serial1_as_invalid() {
        let test_serial = "serial1";
        let validation_result = validate_serial_unique(test_serial);
        assert_eq!(false, validation_result);
    }

    #[test]
    fn validates_new_serial4_as_valid() {
        let test_serial = "serial4";
        let validation_result = validate_serial_unique(test_serial);
        assert_eq!(true, validation_result);
    }
}