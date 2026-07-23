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

    let bytes = isin.as_bytes();
    let mut sum = 0;
    let mut double = true;

    for &byte in bytes[..11].iter().rev() {
        let val = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'A'..=b'Z' => (byte - b'A' + 10) as u32,
            _ => return false,
        };

        sum += luhn_weight(val % 10, double);
        double = !double;

        if val >= 10 {
            sum += luhn_weight(val / 10, double);
            double = !double;
        }
    }

    let last = bytes[11];
    let check_digit = match last {
        b'0'..=b'9' => (last - b'0') as u32,
        _ => return false,
    };

    (sum + check_digit) % 10 == 0
}

#[inline(always)]
fn luhn_weight(digit: u32, double: bool) -> u32 {
    if double {
        let d = digit * 2;
        if d > 9 { d - 9 } else { d }
    } else {
        digit
    }
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
