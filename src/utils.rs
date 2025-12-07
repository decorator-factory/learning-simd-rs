macro_rules! unroll {
    ($num:tt, |$i:ident| $s:stmt) => {
        #[allow(redundant_semicolons, unused_assignments)]
        {
            let mut $i = 0usize;
            $crate::utils::__unroll_inner!($num, |$i| $s);
        }
    };
}
pub(crate) use unroll;

#[rustfmt::skip]
macro_rules! __unroll_inner {
    (0, |$i:ident| $s:stmt) => {};
    (1, |$i:ident| $s:stmt) => { $s; $i += 1; };
    (2, |$i:ident| $s:stmt) => { $crate::utils::__unroll_inner!(1, |$i| $s); $s; $i += 1; };
    (3, |$i:ident| $s:stmt) => { $crate::utils::__unroll_inner!(2, |$i| $s); $s; $i += 1; };
    (4, |$i:ident| $s:stmt) => { $crate::utils::__unroll_inner!(3, |$i| $s); $s; $i += 1; };
    (8, |$i:ident| $s:stmt) => { $crate::utils::__unroll_inner!(4, |$i| $s); $crate::utils::__unroll_inner!(4, |$i| $s); };
    (16, |$i:ident| $s:stmt) => { $crate::utils::__unroll_inner!(8, |$i| $s); $crate::utils::__unroll_inner!(8, |$i| $s); };
}
pub(crate) use __unroll_inner;
