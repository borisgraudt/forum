use crate::error::AppError;

pub fn validation_error(err: validator::ValidationErrors) -> AppError {
    let message = err
        .field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().map(move |e| {
                e.message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("invalid {field}"))
            })
        })
        .next()
        .unwrap_or_else(|| "validation failed".into());
    AppError::BadRequest(message)
}
