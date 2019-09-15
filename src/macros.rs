///
/// A macro to handle those cases where you want to gracefully log
/// or otherwise deal with a `Result::Err`, but cannot use `try!`
/// because your function does not return the `Result` type.
///
/// The expression provided as the first parameter with be matched
/// against the pattern `Err(error)`.  
/// If the pattern matches, the closure provided as the second
/// parameter will be called with the expression result.  
/// The closure result will then be returned from the calling function.
///
/// Helps to prevent unnecessary code indentation by replacing:
///
/// ```
/// # use log::error;
/// # use webtron::unwrap_or_return;
/// # fn main() {
/// # let result: Result<&str, ()> = Ok("ok!");
/// let value = match result {
///     Err(error) => {
///         return error!("{:?}", error);
///     }
///     Ok(value) => value,
/// };
/// # assert_eq!(value, "ok!");
/// # }
/// ```
///
/// with:
///
/// ```
/// # use webtron::unwrap_or_return;
/// # use log::error;
/// # fn main() {
/// # let result: Result<&str, ()> = Ok("ok!");
/// let value = unwrap_or_return!(result, |error| error!("{:?}", error));
/// # assert_eq!(value, "ok!");
/// # }
/// ```
///
#[macro_export]
macro_rules! unwrap_or_return {
    ( $expression:expr ) => {{
        match $expression {
            Err(_) => return,
            Ok(value) => value,
        }
    }};

    ( $expression:expr, $on_error:expr ) => {{
        match $expression {
            Err(error) => return $on_error(error),
            Ok(value) => value,
        }
    }};
}
