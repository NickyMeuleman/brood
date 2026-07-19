use crate::AppError;

/// isin validator that actually matters,
/// the one in the frontend is merely for UX,
/// this one enforces correct data
pub fn validate(isin: &str) -> Result<(), AppError> {
    if isin.len() != 12 {
        return Err(AppError::Validation(format!(
            "ISIN must be exactly 12 characters, got {}: '{isin}'",
            isin.len()
        )));
    }

    let bytes = isin.as_bytes();
    let country_ok = bytes[0..2].iter().all(u8::is_ascii_uppercase);
    let nsin_ok = bytes[2..11]
        .iter()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let check_digit_ok = bytes[11].is_ascii_digit();

    if !country_ok || !nsin_ok || !check_digit_ok {
        return Err(AppError::Validation(format!(
            "ISIN must be 2 uppercase letters, 9 uppercase letters/digits, then 1 digit: '{isin}'"
        )));
    }
    if !luhn_checksum_valid(isin) {
        return Err(AppError::Validation(format!(
            "Invalid ISIN checksum: '{isin}'"
        )));
    }

    Ok(())
}

fn luhn_checksum_valid(isin: &str) -> bool {
    if isin.len() != 12 {
        return false;
    }

    let mut sum = 0;
    let mut double = true; // Start doubling the digit immediately to the left of the check digit

    // Process from right to left, excluding the last character (the check digit)
    // We expand letters on the fly and push digits into a temporary "queue"
    // to process them in the correct order.

    // Collect the 11 digits to process (in reverse order)
    let mut digits = Vec::with_capacity(22);
    for c in isin[..11].chars().rev() {
        let val = match c {
            '0'..='9' => c.to_digit(10).unwrap(),
            'A'..='Z' => (c as u32 - 'A' as u32) + 10,
            _ => return false,
        };

        // Add digits to our list (ones place first, then tens place)
        digits.push(val % 10);
        if val >= 10 {
            digits.push(val / 10);
        }
    }

    // Now apply standard Luhn to the expanded digit list
    for digit in digits {
        let mut n = digit;
        if double {
            n *= 2;
            if n > 9 {
                n -= 9;
            }
        }
        sum += n;
        double = !double;
    }

    let check_digit = isin.chars().last().unwrap().to_digit(10).unwrap();
    (10 - (sum % 10)) % 10 == check_digit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_valid_isin() {
        assert!(validate("IE00BFY0GT14").is_ok());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(validate("IE00BFY0GT1").is_err());
    }

    #[test]
    fn rejects_lowercase() {
        assert!(validate("ie00bfy0gt14").is_err());
    }

    #[test]
    fn rejects_bad_checksum() {
        assert!(validate("IE00BFY0GT15").is_err());
    }
}
