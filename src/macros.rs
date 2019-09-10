///
/// A macro to handle those cases where you want to gracefully
/// deal with a `Result::Err`, but cannot use `try!` because your
/// function does not return the `Result` or `Option` type.
///
/// The expression provided as the first parameter with be matched
/// against the pattern `Err(error)`.  
/// If the pattern matches, the closure provided as the second
/// parameter will be called with the expression result.  
/// The closure result will then be returned from the calling function.
///
/// Helps to prevent unnecessary code indentation by replacing:
///
/// ```ignore
/// let value = match $expression {
///     Err(error) => {
///         return log!(error);
///     }
///     Ok(value) => value,
/// };
/// ```
///
/// with:
///
/// ```ignore
/// let value = unwrap_or_return!($expression, |error| log!(error));
/// ```
///
#[macro_export]
macro_rules! unwrap_or_return {
    ( $expression:expr, $on_error:expr ) => {{
        let result = $expression;
        if let Err(error) = result {
            return $on_error(error);
        }
        result.unwrap()
    }};
}
