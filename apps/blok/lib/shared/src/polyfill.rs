use error_stack::{Context, Report};

use crate::macros::all_the_tuples;

pub trait TupleExt {
    type Context: Context;
    type Ok;

    /// # Errors
    ///
    /// accumulates all errors in the tuple and either returns the errors received or returns the
    /// tuple
    fn fold_reports(self) -> Result<Self::Ok, Report<Self::Context>>;
}

impl TupleExt for () {
    type Context = !;
    type Ok = ();

    fn fold_reports(self) -> Result<Self::Ok, Report<Self::Context>> {
        Ok(())
    }
}

impl<T1, C: Context> TupleExt for (Result<T1, Report<C>>,) {
    type Context = C;
    type Ok = (T1,);

    fn fold_reports(self) -> Result<Self::Ok, Report<Self::Context>> {
        self.0.map(|value| (value,))
    }
}

macro_rules! impl_tuple_ext {
    ([$($elem:ident),*], $other:ident) => {
        #[allow(non_snake_case)]
        impl<C: Context $(, $elem)*, $other> TupleExt for ($(Result<$elem, Report<C>>, )* Result<$other, Report<C>>) {
            type Context = C;
            type Ok = ($($elem ,)* $other);

            fn fold_reports(self) -> Result<Self::Ok, Report<Self::Context>> {
                let ( $($elem ,)* $other ) = self;

                let lhs = ( $($elem ,)* ).fold_reports();

                match (lhs, $other) {
                    (Ok(( $($elem ,)* )), Ok(rhs)) => Ok(($($elem ,)* rhs)),
                    (Ok(_), Err(err)) | (Err(err), Ok(_)) => Err(err),
                    (Err(mut lhs), Err(rhs)) => {
                        lhs.extend_one(rhs);

                        Err(lhs)
                    }
                }
            }
        }
    };
}

all_the_tuples!(impl_tuple_ext);

/// # Errors
///
/// Accumulates all errors in the iterator and either returns the errors received or returns
/// `Ok(())`
pub fn fold_reports<C: Context>(
    reports: impl IntoIterator<Item = Report<C>>,
) -> Result<(), Report<C>> {
    let mut result: Result<(), Report<C>> = Ok(());

    for report in reports {
        match &mut result {
            Err(result) => result.extend_one(report),
            other => *other = Err(report),
        }
    }

    result
}

/// # Errors
///
/// Accumulates all errors in the iterator and either returns the errors received or returns the
/// iterator values as a vector
#[allow(clippy::manual_try_fold)]
pub fn fold_results<T, C: Context>(
    results: impl IntoIterator<Item = Result<T, Report<C>>>,
) -> Result<Vec<T>, Report<C>> {
    results
        .into_iter()
        .fold(Ok(Vec::new()), |acc, result| match (acc, result) {
            (Ok(mut acc), Ok(value)) => {
                acc.push(value);
                Ok(acc)
            }
            (Err(mut acc), Err(error)) => {
                acc.extend_one(error);
                Err(acc)
            }
            (Ok(_), Err(error)) | (Err(error), Ok(_)) => Err(error),
        })
}

pub trait ResultExtend<C>
where
    C: Context,
{
    fn extend_one(&mut self, report: Report<C>);
}

impl<T, C> ResultExtend<C> for Result<T, Report<C>>
where
    C: Context,
{
    fn extend_one(&mut self, report: Report<C>) {
        match self {
            Ok(_) => *self = Err(report),
            Err(error) => error.extend_one(report),
        }
    }
}
