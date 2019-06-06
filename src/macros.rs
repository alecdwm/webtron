///
/// A macro to handle those cases where you want to gracefully
/// deal with a pattern like `Result::Err` or `Option::None`,
/// but cannot use `try!` because your function does not return
/// either of the `Result` or `Option` types.
///
/// The expression provided as the first parameter with be matched
/// against the pattern provided as the second parameter.
/// If the pattern matches, the expression provided as the third
/// parameter will be evaluated and its result returned from the
/// calling function.
///
/// Helps to prevent unnecessary code indentation.
///
#[macro_export]
macro_rules! handle_pat {
    // handle any pattern $p
    ( $e:expr, $p:pat, $if_err:expr ) => {{
        let result = $e;
        if let $p = result {
            return $if_err;
        }
        result.unwrap()
    }};
}

///
/// A macro to handle those cases where you want to gracefully
/// deal with a `Result::Err`, but cannot use `try!` because your
/// function does not return the `Result` type.
///
/// Also helps to prevent unnecessary code indentation.
///
#[macro_export]
macro_rules! handle_err {
    // handle Result::Err
    ( $e:expr, $if_err:expr ) => {{
        let result = $e;
        if let Err(_) = result {
            return $if_err;
        }
        result.unwrap()
    }};

    // handle Result::Err($error) and provide $error to $if_err
    ( $e:expr, $error:ident, $if_err:expr ) => {{
        let result = $e;
        if let Err($error) = result {
            return $if_err;
        }
        result.unwrap()
    }};
}

///
/// A macro to handle those cases where you want to gracefully
/// deal with an `Option::None`, but cannot use `try!` because
/// your function does not return the `Option` type.
///
/// Also helps to prevent unnecessary code indentation.
///
#[macro_export]
macro_rules! handle_none {
    // handle Option::None
    ( $e:expr, $if_none:expr ) => {{
        let option = $e;
        if let None = option {
            return $if_none;
        }
        option.unwrap()
    }};
}
