use crate::transport::{Code, CommandErrorDto};

pub fn new_id() -> Result<String, CommandErrorDto> {
    generate(|bytes| getrandom::fill(bytes).map_err(|_| ()))
}

fn generate(fill: impl FnOnce(&mut [u8]) -> Result<(), ()>) -> Result<String, CommandErrorDto> {
    let mut bytes = [0; 16];
    fill(&mut bytes).map_err(|_| CommandErrorDto::new(Code::IdGenerationFailed))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entropy_failure_is_explicit_and_bytes_encode_without_fallback() {
        assert_eq!(
            generate(|_| Err(())).unwrap_err().code,
            Code::IdGenerationFailed
        );
        assert_eq!(
            generate(|bytes| {
                bytes.fill(0xab);
                Ok(())
            })
            .unwrap(),
            "ab".repeat(16)
        );
    }
}
